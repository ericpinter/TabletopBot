use super::*;
use rand::*;

pub struct StringResult{
    pub output:String,
    pub defurled:String,//the input, but with variables expanded into resolvables. i.e. $x -> d20 +2
    pub unwrapped:String,//with all resolvables (i.e. d20) resolved to numbers
}
impl StringResult{
    fn new(o:String,d:String,u:String) -> StringResult{
        StringResult{output:o,defurled:d,unwrapped:u}
    }
}
pub struct FloatResult{
    pub output:f64,
    pub defurled:String,//the input, but with variables expanded into resolvables. i.e. $x -> d20 +2
    pub unwrapped:String,//with all resolvables (i.e. d20) resolved to numbers
}
impl FloatResult{
    fn new(o:f64,d:String,u:String) -> FloatResult{
        FloatResult{output:o,defurled:d,unwrapped:u}
    }
}


#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to src
struct ArithmeticParser{
    user:String,
    nest_count:u32,//allows up to 100 variable resolves (including recursive)
    repeat_count:u32,//allows up to 100 recursive repeat calls
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
    fn eval(&mut self,expression: Pairs<Rule>) -> FloatResult {
        PREC_CLIMBER.climb(
            expression,
            |pair: Pair<Rule>| match pair.as_rule() {

                Rule::Number => {
                    let result = pair.as_str().parse::<f64>().unwrap();
                    FloatResult::new(result,format!("{}",result),format!("{}",result))
                },
                Rule::Ternary => {// in the form t?x:y. x is the value of this statement if x is non-zero, y is the value if it is zero
                    let mut stmt = pair.into_inner();
                    let t = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    println!("Ternary on {}",t);
                    let x = stmt.next().unwrap();
                    let y = stmt.next().unwrap();
                    if t!=0.0 {
                        self.eval(Pairs::single(x))
                    } else {
                        self.eval(Pairs::single(y))
                    }
                },
                Rule::Norm =>{// in the form xdy e.g. 3d6
                    let mut stmt = pair.into_inner();
                    let x = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    let y = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    //x and y will not have String components
                    let mut sum:u64 =0;
                    for _ in 0..(x as u32) {
                        sum+=rand::thread_rng().gen_range(1,y as u64+1);
                    }
                    FloatResult::new(sum as f64,format!("{}d{}",x,y),format!("{}",sum))
                },
                Rule::Shortnorm =>{//e.g. d20
                    let mut stmt = pair.into_inner();
                    let y = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    let r = rand::thread_rng().gen_range(1,y as u64+1) as f64;

                    FloatResult::new(r,format!("d{}",y),format!("{}",r))
                },
                Rule::Keep =>{//e.g. 3d6k2
                    let mut stmt = pair.into_inner();
                    let x = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    let y = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    let k = self.eval(Pairs::single(stmt.next().unwrap())).output;
                    let mut nums:Vec<u64> = vec![0;x as usize];
                    for i in 0..(x as usize) {
                        nums[i]=rand::thread_rng().gen_range(1,y as u64+1);
                    }
                    nums.sort();
                    let sum:u64 = nums.iter().rev().take(k as usize).sum();

                    FloatResult::new(sum as f64,format!("{}d{}k{}",x,y,k),format!("{}",sum))
                },
                Rule::Var => {
                    if self.nest_count > 100 { FloatResult::new(0.0,String::new(),String::new()) } else {
                        self.nest_count += 1;
                        let raw_name = pair.as_str().to_string();
                        let i = pair.into_inner().next();
                        println!("{:?}",i);
                        //println!("{:?}",i.unwrap().as_rule());
                        let v_name = match i {
                            Some(q) => {
                                format!("${}",self.string_eval(Pairs::single(q)).output)
                            },
                            //we only need to check if our variable is being indirectly mentioned (e.g. $("x"+3) => $x3). Otherwise it's just a normally named one
                            None => {raw_name}
                        };
                        println!("Resolving variable named {}",v_name);
                        match resolve(&self.user, &v_name) {
                            Some(s) => {
                                let result = ArithmeticParser::parse(Rule::Arithmetic, &s).expect("Failed to parse");
                                self.eval(result)
                            },

                            None => FloatResult::new(0.0,String::from(""),String::from(""))
                        }
                    }

                },
                Rule::Negate => {
                    let val = self.eval(pair.into_inner());
                    FloatResult::new(-val.output,format!("-{}",val.defurled),format!("-{}",val.unwrapped))
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

                    let result = self.eval(val_group.into_inner());
                    //self.unwrapped.push_str(&format!("{}",result));
                    result
                },
                Rule::BackEq =>{
                    //here we want to evaluate the expression on the left and then assign it to the variable named on the right
                    let mut parts = pair.into_inner();
                    let val = self.eval(parts.next().unwrap().into_inner());
                    let var = parts.next().unwrap().as_str();
                    //println!("setting {} to {}",var, val);
                    set_var(&self.user,var,&format!("{}",val.output));
                    //self.unwrapped.push_str(&format!("{}",val));
                    val
                },
                /*Passthrough*/ Rule::Arithmetic | Rule::Calculation | Rule::TextExp |Rule::TextBasic| Rule::Exp | Rule::MathExp | Rule::Roll | Rule::PriExp => self.eval(pair.into_inner()),

                Rule::EOI =>{FloatResult::new(0.0,String::new(),String::new())},
                _ => {println!("Failed on: {:?} {:?}",pair.as_rule(),pair);unreachable!()},
            },
            |lhs: FloatResult, op: Pair<Rule>, rhs: FloatResult| {
                let o = match op.as_rule() {
                    Rule::add      => {lhs.output + rhs.output},
                    Rule::subtract => {lhs.output - rhs.output},
                    Rule::multiply => {lhs.output * rhs.output},
                    Rule::divide   => {lhs.output / rhs.output},
                    Rule::power    => {lhs.output.powf(rhs.output)},
                    _ => unreachable!()};
                FloatResult::new(o,format!("{}{}{}",lhs.defurled,op.as_str(),rhs.defurled),format!("{}{}{}",lhs.unwrapped,op.as_str(),rhs.unwrapped))
            },
        )
    }
    fn string_eval(&mut self, expression: Pairs<Rule>) -> StringResult {
        PREC_CLIMBER.climb(
            expression,
            |first: Pair<Rule>|
                match first.as_rule() {
                    Rule::Arithmetic | Rule::Calculation => {
                        self.string_eval(first.into_inner())
                    },
                    Rule::Ternary => {// in the form t?x:y. x is the value of this statement if x is non-zero, y is the value if it is zero
                        let mut stmt = first.into_inner();
                        let t = self.eval(Pairs::single(stmt.next().unwrap())).output;
                        println!("Ternary on {}",t);
                        let x = stmt.next().unwrap();
                        let y = stmt.next().unwrap();
                        if t!=0.0 {
                            self.string_eval(Pairs::single(x))
                        } else {
                            self.string_eval(Pairs::single(y))
                        }
                    },
                    Rule::Exp => {
                        let s = self.eval(first.into_inner());
                        StringResult::new(s.output.to_string(),s.defurled,s.unwrapped)
                    },
                    Rule::TextExp | Rule::TextBasic => self.string_eval(first.into_inner()),
                    Rule::String => {
                        //everything but the surrounding " and ", with newlines
                        let s=first.as_str();
                        StringResult::new(s[1..s.len()-1].to_string().replace("\\n","\n"),String::new(),String::new())
                    },
                    Rule::Repeat => {
                        if self.repeat_count>100 {StringResult::new(String::new(),String::new(),String::new())}
                        else{
                            let mut stmt = first.into_inner();
                            let e = Pairs::single(stmt.next().unwrap());
                            println!("{}",e);
                            let r = self.eval(Pairs::single(stmt.next().unwrap())).output;
                            let nums:Vec<String> = vec![String::new(); r as usize].iter().map(|_|
                                {
                                    //let mut nest_parser = ArithmeticParser{user:self.user.clone(),nest_count:self.nest_count,repeat_count:self.repeat_count};
                                    //let res = nest_parser.string_eval(e.clone()).0;
                                    let res = self.string_eval(e.clone()).output;
                                    println!("{}",res);
                                    res
                                }
                            ).collect();
                            let s= nums.join(", ");
                            StringResult::new(format!("({})",&s),String::new(),String::new())
                        }
                    },
                    _ =>{
                        let val = self.eval(first.into_inner());
                        StringResult::new(val.output.to_string(),val.defurled,val.unwrapped)
                    },
                },
            |lhs: StringResult, _op: Pair<Rule>, rhs: StringResult| {
                StringResult::new(format!("{}{}",lhs.output,rhs.output),format!("{}{}",lhs.defurled,rhs.defurled),format!("{}{}",lhs.unwrapped,rhs.unwrapped))
            },
        )
        //TODO switch to returning using self, letting calling function control defurl, unwrap, out.
    }

}
pub fn parse(user:String,s:String)->std::result::Result<StringResult,Error>{

    let now = Instant::now();
    let result = ArithmeticParser::parse(Rule::Arithmetic,&s).expect("Failed to Parse");
    println!("{:?}",result);
    let mut parser = ArithmeticParser{user,nest_count:0,repeat_count:0};
    let val = parser.string_eval(result);
    println!("parsing and calcing took {} ms",now.elapsed().as_millis());
    println!("output is: {}",val.output);
    println!("defurled is: {}",val.defurled);
    println!("unwrapped is: {}",val.unwrapped);
    Result::Ok(val)

}
