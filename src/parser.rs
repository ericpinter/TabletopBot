use super::*;
use rand::*;

use self::Variable::*;
use pest::pratt_parser::*;
use pest::iterators::Pairs;
use pest::iterators::Pair;
use std::fmt::{Display, Formatter};
use std::fmt;
use std::convert::TryInto;

#[derive(Debug)]
pub enum Variable {
    Text(String),
    Num(f64),
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_string())
    }
}

impl Variable {
    fn into_string(&self) -> String {
        match self {
            Text(s) => s.to_owned(),
            Num(n) => format!("{}", n)
        }
    }

    fn as_num(self) -> Result<f64, String> {
        match self {
            Text(_) => Err(String::from("Cannot use text as a number")),
            Num(n) => Ok(n)
        }
    }
}

fn lift(f:Box<dyn Fn(f64,f64) -> f64>) -> Box<dyn Fn(Variable,Variable) -> Variable> {
    Box::new(move |a,b| 
        match (a,b) {
            (Num(l),Num(r)) => Num(f(l,r)),
            (l,r) => Text(format!("{} ,,, {}",l,r))
        }
    )
}


impl<T> From<T> for Variable where T: Into<f64> {
    fn from(x: T) -> Self {
        Num(x.into())
    }
}

#[derive(Debug)]
pub struct Calculation {
    pub output: Variable,
    pub defurled: String,
    //the input, but with variables expanded into resolvables. i.e. $x -> d20 +2
    pub unwrapped: String,//with all resolvables (i.e. d20) resolved to numbers
    pub nest_count: u32,
    //allows up to 100 variable resolves (including recursive)
    pub repeat_count: u32,//allows up to 100 recursive repeat calls
}

impl Display for Calculation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.output, self.defurled, self.unwrapped)
    }
}

impl Calculation {
    fn new<T>(v: T, d: String, u: String) -> Calculation where T: Into<Variable> {
        Calculation { output: v.into(), defurled: d, unwrapped: u, nest_count:0, repeat_count:0 }
    }
    fn empty_num() -> Calculation {
        Calculation::new(0.0, String::new(), String::new())
    }

    fn empty_str() -> Calculation {
        Calculation::new(Text(String::new()), String::new(), String::new())
    }

    fn combine(self,other:Self, f: Box<dyn Fn(Variable,Variable) -> Variable>) -> Self {
        let v = f(self.output,other.output);

        Calculation {
            output:v,
            defurled:format!("{} ,,, {}",self.defurled,other.defurled),
            unwrapped:format!("{} ,,, {}",self.unwrapped,other.unwrapped),
            nest_count:self.nest_count+other.nest_count,
            repeat_count:self.repeat_count+other.repeat_count
        }
    }
}

type CalcResult = Result<Calculation, String>;


#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to src
struct ArithmeticParser {
    user: String,
}

fn as_slice<P, T, const N: usize>(pairs: P) -> Result<[T; N], String> where P: Iterator<Item=Result<T, String>> {
    let v = pairs.collect::<Result<Vec<T>, _>>()?;
    v.try_into().map_err(|_| "Failed conversion".into())
}

//simple loop to strike out all the characters in a string. Probably horribly inefficient for large strings because of the allocating but fine in this use case where we only use it for numbers (maybe 20 significant digits)
fn strikethrough(s: String) -> String {
    let mut strike = String::new();
    strike.reserve(2 * s.len());
    let chars = s.chars();
    for c in chars {
        strike.push(c);
        strike.push(char::from_u32(0x336).unwrap());
    }
    strike
}

impl ArithmeticParser {
    pub fn parse_infix(&self, lhs: CalcResult, op: Pair<Rule>, rhs: CalcResult) -> CalcResult  {
        println!("{:?} {:?}", lhs, rhs);
        let l = lhs?;
        let r = rhs?;
        
        match op.as_rule() {
            Rule::add => { Ok(l.combine(r, lift(Box::new(|a,b|a+b)))) }
            Rule::subtract => { Ok(l.combine(r, lift(Box::new(|a,b|a-b)))) }
            Rule::multiply => { Ok(l.combine(r, lift(Box::new(|a,b|a*b)))) }
            Rule::divide => { Ok(l.combine(r, lift(Box::new(|a,b|a/b)))) }
            Rule::power => { Ok(l.combine(r, lift(Box::new(|a,b|a.powf(b))))) }
            _ => unreachable!()
        }
    }
    
    pub fn parse_primary(&self, pair: Pair<Rule>, pratt:&PrattParser<Rule>) -> CalcResult {
        match pair.as_rule() {
        Rule::Number => {
            let result = pair.as_str().parse::<f64>().unwrap();
            Ok(Calculation::new(Num(result), format!("{}", result), format!("{}", result)))
        }
        Rule::Ternary => {// in the form t?x:y. x is the value of this statement if t is non-zero, y is the value if it is zero
            let stmt = pair.into_inner();
            let [t_e, x, y] = as_slice(stmt.map(|x| Ok(x)))?;
    
            let t = self.parse_expr_pair(t_e,pratt)?.output;
            println!("Ternary on {:?}", t);
    
            match t {
                Num(x) if x < 1.0 && x > -1.0 => self.parse_expr_pair(y,pratt),
                Text(_) => self.parse_expr_pair(y,pratt),
                _ => self.parse_expr_pair(x,pratt)
            }
        }
        Rule::Norm => {// in the form xdy e.g. 3d6
            let stmt = pair.into_inner();
            let [x, y] = as_slice(stmt.map(|val| self.parse_expr_pair(val,pratt)?.output.as_num()))?;
    
            //x and y will not have String components
            let mut sum: u64 = 0;
    
            for _ in 0..(x as u32) {
                sum += rand::thread_rng().gen_range(1, y as u64 + 1);
            }
            Ok(Calculation::new(sum as f64, format!("{}d{}", x, y), format!("{}", sum)))
        }
        Rule::Shortnorm => {//e.g. d20
            let mut stmt = pair.into_inner();
            let y = self.parse_expr_pair(stmt.next().unwrap(),pratt)?.output.as_num()?;
    
            let r = rand::thread_rng().gen_range(1, y as u64 + 1) as f64;
    
            Ok(Calculation::new(Num(r), format!("d{}", y), format!("{}", r)))
        }
    
        Rule::Keep => {//e.g. 3d6k2
            let stmt = pair.into_inner();
    
            let [x, y, k] = as_slice(stmt.map(|val| self.parse_expr_pair(val,pratt)?.output.as_num()))?;
            let mut nums: Vec<u64> = vec![0; x as usize];
            for i in 0..(x as usize) {
                nums[i] = rand::thread_rng().gen_range(1, y as u64 + 1);
            }
            nums.sort();
            let sum: u64 = nums.iter().rev().take(k as usize).sum();
            //todo make unwrapped here use the individual rolls, with strike-throughs
            let summary = nums.iter().enumerate().map(|(ind, val)| if ind < (x - k) as usize { strikethrough(val.to_string()) } else { val.to_string() }).collect::<Vec<String>>().join(" + ");
    
            Ok(Calculation::new(sum as f64, format!("{}d{}k{}", x, y, k), format!("({})", summary)))
        }
    
        Rule::Var => {
            let raw_name = pair.to_string();
            let i = pair.into_inner().next().unwrap();

            println!("i is {:?}", i);

            let v_name = match i.as_rule() {
                Rule::Calculation => {
                    format!("${}", self.parse_expr_pair(i,pratt)?.output.into_string())
                }
                //we only need to check if our variable is being indirectly mentioned (e.g. $("x"+3) => $x3). Otherwise it's just a normally named one
                Rule::Identifier => { raw_name }
                _ => { return Err(String::from("impossible")); }
            };

            println!("Resolving variable named {}.", v_name);
            match resolve(&self.user, &v_name) {
                Some(s) => {
                    let result = ArithmeticParser::parse(Rule::Arithmetic, &s).expect("Failed to parse");
                    self.parse_and_validate(result,pratt)
                }
                None => Ok(Calculation::empty_num())
            }
        }
        Rule::Negate => {
            let val = self.parse_expr_pair(pair,pratt)?;
            Ok(Calculation::new(-val.output.as_num()?, format!("-{}", val.defurled), format!("-{}", val.unwrapped)))
        }
        Rule::FrontEq => {
            //here we assign the variable on the left to the *raw* value of the expression on the right. i.e. it will be re-calculated when the variable is used
            //we then calculate and return what the expression was
            //this lets us assign variables inside of larger expressions e.g. ($x=d20+6)-2
            let stmt = pair.into_inner();
    
            let [var_p, val_group] = as_slice(stmt.map(|x| Ok(x)))?;
            let var = var_p.as_str();
            let val = val_group.as_str();
            println!("setting {} to {}", var, val);
            set_var(&self.user, var, val);
    
            self.parse_expr_pair(val_group,pratt)
        }
        Rule::BackEq => {
            //here we want to evaluate the expression on the left and then assign it to the variable named on the right
            let mut parts = pair.into_inner();
            let val = self.parse_and_validate(parts.next().unwrap().into_inner(),pratt)?;
            let var = parts.next().unwrap().as_str();
            //println!("setting {} to {}",var, val);
            set_var(&self.user, var, &format!("{}", val.output));
            //self.unwrapped.push_str(&format!("{}",val));
            Ok(val)
        }
        Rule::String => {
            //everything but the surrounding " and ", with newlines
            let s = pair.as_str();
            println!("have string {}", s);
            let text = s[1..s.len() - 1].to_string().replace("\\n", "\n");
            Ok(Calculation::new(Text(text.clone()), text.clone(), text))
        }
        Rule::Repeat => {
            let mut stmt = pair.into_inner();

            let e = Pairs::single(stmt.next().unwrap());
            let r = self.parse_expr_pair(stmt.next().unwrap(),pratt)?.output.as_num()? as usize;
            let mut nums: Vec<String> = Vec::new();

            for _ in 0..r {
                let res = self.parse_and_validate(e.clone(),pratt)?.output;//TODO figure out something better for restricting repeat count on these calls
                println!("{}", res);
                nums.push(res.into_string())
            }
            let s = nums.join(", ");
            Ok(Calculation::new(Text(format!("({})", &s)), String::new(), String::new()))
        }
    
        //passthrough
        Rule::Arithmetic | Rule::Calculation | Rule::Exp | Rule::MathExp |
        Rule::TextBasic | Rule::PriExp | Rule::Roll => {
            self.parse_and_validate(pair.into_inner(),pratt)
        }
    
        _ => {
            eprintln!("failed at {}", pair.to_string());
            unreachable!()
        }
    }
    }


    

    fn parse_expr_pair(&self, pair:Pair<Rule>, pratt: &PrattParser<Rule>) -> CalcResult {
        self.parse_and_validate(Pairs::single(pair), pratt)
    }
    
    fn parse_expr(&self, pairs: Pairs<Rule>, pratt: &PrattParser<Rule>) -> CalcResult {
        pratt
            .map_primary(|primary| self.parse_primary(primary, pratt))
            .map_infix(|lhs, op, rhs| self.parse_infix(lhs, op, rhs))
            .map_postfix(|lhs, op| match op.as_rule() {
                Rule::EOI => lhs,
                _ => unreachable!()
            })
            .parse(pairs)
    }
    

    fn parse_and_validate(&self, pairs:Pairs<Rule>, pratt:&PrattParser<Rule>) -> CalcResult {
        let cr = self.parse_expr(pairs, pratt)?;

        if cr.repeat_count > 100 {
            Ok(Calculation::empty_num())
        } else if cr.nest_count > 100 {
            Ok(Calculation::empty_num())
        } else { Ok(cr) }
    }
    
}


pub fn parse(user: String, s: &str) -> CalcResult {
    let now = Instant::now();

    let result = ArithmeticParser::parse(Rule::Arithmetic, s).expect("Failed to Parse");
    println!("got {}", result);
    
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::subtract, Assoc::Left))
        .op(Op::infix(Rule::multiply, Assoc::Left) | Op::infix(Rule::divide, Assoc::Left))
        .op(Op::infix(Rule::power, Assoc::Right))
        .op(Op::postfix(Rule::EOI));//Bug? https://github.com/pest-parser/pest/issues/943;

    
    let parser = ArithmeticParser { user };
    let val = parser.parse_and_validate(result, &pratt)?;
    println!("parsing and calcing took {} ms", now.elapsed().as_millis());
    println!("output is: {}", val.output);
    println!("defurled is: {}", val.defurled);
    println!("unwrapped is: {}", val.unwrapped);
    Ok(val)
}

pub fn is_valid(s: &str) -> bool {
    ArithmeticParser::parse(Rule::Arithmetic, s).is_ok()
}
