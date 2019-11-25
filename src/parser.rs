use super::*;
use rand::*;

#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to src
struct ArithmeticParser{
    user:String,
    //TODO fully implement this
    defurled:String,//the input, but with variables expanded into resolvables
    //TODO fully implement this
    unwrapped:String,//with all resolvables (i.e. d20) resolved to numbers
    output:String,//The thing the user cares about.
    nest_count:u32,//allows up to 100 variable resolves (including recursive)
    repeat_count:u32,//allows up to 100 recursive repeat calls
    //calling either eval function will return something very similar to the output field, but not formatted for display
}
use pest::prec_climber::*;
use pest::iterators::Pairs;
use pest::iterators::Pair;

lazy_static! {
    static ref PREC_CLIMBER: PrecClimber<Rule> = {
        use Rule::*;
        use pest::prec_climber::Assoc::*;

        PrecClimber::new(vec![
            Operator::new(add, Left) | Operator::new(subtract, Left),
            Operator::new(multiply, Left) | Operator::new(divide, Left),
            Operator::new(power, Right)
        ])
    };
}

impl ArithmeticParser{
    fn eval(&mut self,expression: Pairs<Rule>) -> f64 {
        PREC_CLIMBER.climb(
            expression,
            |pair: Pair<Rule>| match pair.as_rule() {

                Rule::Number => pair.as_str().parse::<f64>().unwrap(),

                Rule::Norm =>{// in the form xdy e.g. 3d6
                    let mut stmt = pair.into_inner();
                    let x = self.eval(Pairs::single(stmt.next().unwrap())) ;
                    //stmt.next();
                    let y = self.eval(Pairs::single(stmt.next().unwrap()));
                    let mut sum:u64 =0;
                    for _ in 0..(x as u32) {
                        sum+=rand::thread_rng().gen_range(1,y as u64+1);
                    }
                    sum as f64
                },
                Rule::Shortnorm =>{//e.g. d20
                    let y = self.eval(Pairs::single(pair.into_inner().next().unwrap()));
                    rand::thread_rng().gen_range(1,y as u64+1) as f64
                },
                Rule::Keep =>{//e.g. 3d6k2
                    let mut stmt = pair.into_inner();
                    let x = self.eval(Pairs::single(stmt.next().unwrap()));
                    //stmt.next();
                    let y = self.eval(Pairs::single(stmt.next().unwrap()));
                    let k = self.eval(Pairs::single(stmt.next().unwrap()));
                    let mut nums:Vec<u64> = vec![0;x as usize];
                    for i in 0..(x as usize) {
                        nums[i]=rand::thread_rng().gen_range(1,y as u64+1);
                    }
                    nums.sort();
                    let sum:u64 = nums.iter().rev().take(k as usize).sum();
                    sum as f64
                },
                Rule::Var => {
                    if self.nest_count > 100 { 0.0 } else {
                        self.nest_count += 1;

                        match resolve(&self.user, pair.as_str()) {
                            Some(s) => {
                                let result = ArithmeticParser::parse(Rule::Arithmetic, &s).expect("Failed to parse");

                                let mut nest_parser = ArithmeticParser{user:self.user.clone(),unwrapped:String::new(),defurled:String::new(),output:String::new(),nest_count:self.nest_count,repeat_count:self.repeat_count};
                                nest_parser.eval(result)
                            },

                            None => 0.0
                        }
                    }

                },
                Rule::Negate => {
                    -self.eval(pair.into_inner())
                },
                Rule::FrontEq =>{
                    //here we assign the variable on the left to the *raw* value of the expression on the right. i.e. it will be re-calculated when the variable is used
                    //we then calculate and return what the expression was
                    //this lets us assign variables inside of larger expressions e.g. ($x=d20+6)-2
                    let mut parts = pair.into_inner();
                    let var = parts.next().unwrap().as_str();
                    let val_group = parts.next().unwrap();
                    let val = val_group.as_str();
                    println!("setting {} to {}",var, val);
                    set_var(&self.user,var,val);
                    self.eval(val_group.into_inner())
                },
                Rule::BackEq =>{
                    //here we want to evaluate the expression on the left and then assign it to the variable named on the right
                    let mut parts = pair.into_inner();
                    let val = self.eval(parts.next().unwrap().into_inner());
                    let var = parts.next().unwrap().as_str();
                    println!("setting {} to {}",var, val);
                    set_var(&self.user,var,&format!("{}",val));
                    val
                },
                /*Passthrough*/ Rule::Arithmetic | Rule::TextExp |Rule::TextBasic| Rule::Exp | Rule::MathExp | Rule::Roll | Rule::PriExp => self.eval(pair.into_inner()),

                Rule::EOI =>{0.0},
                _ => {println!("Failed on: {:?} {:?}",pair.as_rule(),pair);unreachable!()},
            },
            |lhs: f64, op: Pair<Rule>, rhs: f64| {
                let o = match op.as_rule() {
                    Rule::add      => {lhs + rhs},
                    Rule::subtract => {lhs - rhs},
                    Rule::multiply => {lhs * rhs},
                    Rule::divide   => {lhs / rhs},
                    Rule::power    => {lhs.powf(rhs)},
                    _ => unreachable!()};
                //self.output+=&format!("{}",o);
                o
            },
        )
    }
    fn string_eval(&mut self, expression: Pairs<Rule>) -> String{
        PREC_CLIMBER.climb(
            expression,
            |first: Pair<Rule>|
                match first.as_rule() {
                    Rule::Arithmetic => {
                        let s = self.string_eval(first.into_inner()).to_string();
                        self.output+=&s;
                        s
                    },
                    Rule::Exp => self.eval(first.into_inner()).to_string(),
                    Rule::TextExp => {
                        self.string_eval(first.into_inner())
                    },
                    Rule::TextBasic =>self.string_eval(first.into_inner()),
                    Rule::String => {
                        //everything but the surrounding " and ", with newlines
                        let s=first.as_str();
                        s[1..s.len()-1].to_string().replace("\\n","\n")
                    },
                    Rule::Repeat => {
                        if self.repeat_count>100 {String::new()}
                        else{
                            let mut stmt = first.into_inner();
                            let e = Pairs::single(stmt.next().unwrap());
                            println!("{}",e);
                            let r = self.eval(Pairs::single(stmt.next().unwrap())) as usize;
                            let nums:Vec<String> = vec![String::new();r].iter().map(|_|
                                {
                                    let mut nest_parser = ArithmeticParser{user:self.user.clone(),unwrapped:String::new(),defurled:String::new(),output:String::new(),nest_count:self.nest_count,repeat_count:self.repeat_count};
                                    let res = nest_parser.string_eval(e.clone());
                                    println!("{}",res);
                                    res
                                }
                            ).collect();
                            let s= nums.join(", ");
                            format!("({})",&s)
                        }
                    },
                    _ =>{self.eval(first.into_inner()).to_string()},
                },
            |lhs: String, _op: Pair<Rule>, rhs: String| {
                format!("{}{}",lhs,rhs)
            },
        )
        //TODO switch to returning using self, letting calling function control defurl, unwrap, out.
    }

}
pub fn parse(user:String,s:String)->std::result::Result<String,String>{

    let now = Instant::now();
    let result = ArithmeticParser::parse(Rule::Arithmetic,&s).expect("Failed to Parse");
    println!("{:?}",result);
    let mut parser = ArithmeticParser{user,unwrapped:String::new(),defurled:String::new(),output:String::new(),nest_count:0,repeat_count:0};
    parser.string_eval(result);
    println!("parsing and calcing took {} ms",now.elapsed().as_millis());
    println!("output is: {}",parser.output);
    println!("defurled is: {}",parser.defurled);
    println!("unwrapped is: {}",parser.unwrapped);
    Result::Ok(parser.output)

}
