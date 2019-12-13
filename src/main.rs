extern crate serenity;
extern crate regex;
extern crate pest;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate serde_json;
extern crate rand;
extern crate reqwest;

mod database;
mod parser;
use parser::parse;
use pest::*;

use database::*;
use std::env;
use serenity::{
    model::{channel::Message, gateway::Ready,id::ChannelId},
    prelude::*,
};
use std::time::Instant;

struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot{return;}
        let reply_info = (msg.author.mention(),msg.channel_id);
        let mut args = msg.content.split_whitespace();
        let mut first_input = args.next().unwrap_or(" ").chars();

        println!("Recieved input");

        if let Some('!') = first_input.next() {
            let now = Instant::now();
            let cmd_string = first_input.collect::<String>().to_lowercase();
            let cmd = cmd_string.as_str();//ignore the inital !
            let user = msg.author.id.0.to_string();

            println!("cmd is {} matching at {}",cmd,now.elapsed().as_millis());
            match cmd {

                "port" => {
                    let num = args.next();
                    let response = match num {
                        Some(id) => {port_character(&user, &id)}
                        None => {String::from("It appears you gave an invalid mythweavers id")}
                    };
                    reply(reply_info, ctx,&response);
                }

                "char" => {
                    let name = args.collect::<Vec<&str>>().join(" ");
                    add_char(&user, &name);
                    reply(reply_info,ctx,&format!("Created a char named: {} ",name));
                }

                "switch" => {
                    let name = args.collect::<Vec<&str>>().join(" ");
                    reply(reply_info,ctx,&set_cc(&user, &name));
                }

                "lc" | "lchar" | "listchar" | "listchars" | "charlist" => {
                    reply(reply_info,ctx, &list_chars(user));
                }

                "delchar" => {
                    let name = args.collect::<Vec<&str>>().join(" ");
                    reply(reply_info,ctx, &remove_char(&user, &name));
                }

                "h" | "help" => {
                    reply(reply_info,ctx, "![h/help] you should know what this does\n![listchar/listchars] will list the valid characters you have defined\n![char] [name] will create a blank character\n![delchar] [name] will remove said character\n![switch] [name] switch to the given characters\n![port] [id] will port an entire mythweavers sheet (or at least the important stuff) given the number at the end of the url\n\
                    ![a/assign] $[varname] [expression] will assign a variable. Note that variables *can* be used in expressions\n![l/list] lists the variables your current character has\n![rename] $[varname] $[varname] deletes the first var and puts its value in the second\n![r/roll] [expression] will return the result of that roll\n![re/rollexplicit] [expression] will act like a roll, but tell you the intermediate values\n![v/value] $[varname] will print what the roll system thinks of a variable as\n![c/clear] $[varname] will delete a variable\n![a/assign] $[varname] [expression] will assign the variable to that expression (and recompute that expression every time the variable is resolved)\n\
                    ![ic/incharacter] [text] will make this bot print the text in a way that shows your character said it. If you have a discord emote which has the same name as your $character variable, it will include that icon.\nValid expressions are combinations of basic Arithmetic operations, numbers, rolls (e.g. 3d6), and variables\nCool roll expressions include $[varname], [x]d[y]k[z] (e.g. 4d6k3), the repeat block\nrepeat([expression],[number]) which will quit after your call invokes more than 100 repeats, and inline var assignment (e.g $x=d20+2 or $x=$y). If a variable takes more than 40 variable resolves before it itself resolves, any further nested variables are treated as 1\nNote: the expression \"$x = d20\" will reroll that value each time $x is used, but \"d20 = $x\" will store the result of that roll and not recalculate. They can also be used in larger expressions (e.g. \"d20 + $health = d6\").");
                }

                //We've handled all of the commands which don't require characters, or for which chars are specified. The rest are contextual, and so require a valid currentChar
                _ => {
                    if valid_cc(&user) {
                        match cmd {
                            "l" | "list" => { reply(reply_info,ctx, &list_vars(&user)); }
                            "v" | "value" => {
                                //TODO be wary of unwrapping and resolution before
                                let name = args.next().unwrap();
                                let val = match resolve(&user,name){
                                    Some(s)=>{s},
                                    None =>{String::from("Does not exist!")}
                                };

                                reply(reply_info,ctx, &format!("{} : {}",name,val)  );
                            }
                            "c" |  "clear" => {
                                let val = args.next().unwrap().to_string();
                                reply(reply_info,ctx, &remove_var(&user, &val))  ;
                            }
                            "a" | "assign" => {
                                let var = args.next().unwrap().to_string();
                                if regex::Regex::new(r"\$[[a-zA-Z]\d_()]").unwrap().is_match(&var){

                                    let exp =  args.collect::<Vec<&str>>().join(" ");

                                    //TODO test exp with grammar

                                    set_var(&user, &var, &exp);
                                    reply(reply_info,ctx, "Assigned!");

                                }
                                else{ reply(reply_info,ctx, "Please start all vars with $, and use only a-z A-Z _ 0-9 and () in the variable's name"); }
                            }
                            "ic" | "incharacter" => {
                                let out = match resolve(&user,"$character"){
                                    Some(s) =>{s},
                                    None => {msg.author.name.clone()},
                                };


                                let color =
                                    if let Some(s) = resolve(&user,"$color") {
                                        match s.parse::<u32>() {
                                            Ok(n) =>{n},
                                            Err(_) => {123456},
                                        }
                                    }
                                    else {123456};


                                let partial_guild = msg.guild_id.unwrap().to_partial_guild(&ctx).unwrap();

                                let icon_url =
                                    match partial_guild.emojis.values().find(|e| {e.name == out}) {
                                        //try and find a custom emoji named after their character
                                        Some(icon) =>{icon.url()}
                                        None =>{
                                            //otherwise try to just make it their avatar
                                             msg.author.avatar_url().unwrap_or(
                                                 //otherwise give them something universal
                                                 String::from("https://modworkshop.net/mydownloads/previews/preview_54895_1540694735_b03cf8b0fc082142d5ab1ff8a7dc0fb4.jpg"))
                                        }
                                    };

                                let text =  args.collect::<Vec<&str>>().join(" ");
                                let result = msg.channel_id.send_message(&ctx, |m| {
                                    m.embed(|e| {
                                            e.title(format!("{}",text))
                                                .author(|aut| aut.name(&out).icon_url(icon_url) )
                                                .color(color)
                                        })
                                });
                                println!("{:?}",result);
                                let _ = msg.delete(&ctx);
                            }

                            "r" | "roll" => {
                                let out = match parse(user,args.collect::<Vec<&str>>().join(" ")){
                                    Ok(parse_result) =>{parse_result.output},
                                    Err(_) =>{String::from("Invalid Input")}
                                };
                                reply(reply_info,ctx,&out);
                            }

                            "re" | "rollexplicit" => {
                                let out = match parse(user,args.collect::<Vec<&str>>().join(" ")){
                                    Ok(parse_result) =>{
                                        format!("({}) → ({}) → {}",parse_result.defurled,parse_result.unwrapped,parse_result.output)
                                    },
                                    Err(_) =>{String::from("Invalid Input")}
                                };
                                reply(reply_info,ctx,&out);
                            }

                            _ => { reply(reply_info,ctx,"Unknown Command") }
                        }
                    } else {
                        reply(reply_info,ctx,"It seems that you're trying to use the bot with an invalid active character (or perhaps with no characters at all)! Port one from MW with 'port', create a blank one with 'char', or use 'switch' to switch to an existing one.");
                    }
                }
            }
            println!("Answering this request took {} millis\n",now.elapsed().as_millis());
        }
    }

    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

 fn port_character(user:&str,num:&str) -> String {
    if regex::Regex::new(r"^[0-9]+$").expect("Regex failed").is_match(num){
        let url = format!("https://www.myth-weavers.com/api/v1/sheets/sheets/{}",num);
        let response = reqwest::blocking::get(&url).expect("Request failed");
        let json:serde_json::Value = response.json().expect("Request worked, but the given sheet has no json");
        //Mythweavers formats their data like this, don't blame me for the mess
        let sheet_template = json["sheetdata"]["sheet_template_id"].as_u64().expect("Failed to find template");

        let map_str = json["sheetdata"]["sheet_data"]["jsondata"].as_str().expect("Failed to traverse json");
        let map:serde_json::Value = serde_json::from_str(map_str).expect("failed to parse nested json");
        let map = map.as_object().expect("Map");

        let char_name = map.get("Name").expect("Name not found").as_str().expect("finding char_name");
        println!("Making a character named {}",char_name);
        add_char(user, char_name);

        match sheet_template{
            11 => {port_35e(user,map); list_vars(user)},
            43 => {port_sf(user,map); list_vars(user)},
            other =>{
                println!("Failed to port {:?}",other);
                String::from("Are you sure that this is a sheet of the right type? It may not be supported at the moment")
            }
        }

    } else {String::from("Please make sure you have only the number at the end of your mw sheet in this command")}
}

fn port_35e (user:&str,m:&serde_json::Map<String,serde_json::Value>){
    println!("Starting the port!");
    let set=|k:&str,v:&str| set_var(user, &format!("${}", k), &format!("d20{}{}", if v.parse::<i64>().unwrap()>=0 {"+"} else {""}, v));

    let skill_regex = regex::Regex::new("^Skill[0-9]{2}$").expect("Wrong RegEx");
    //port over all skills
    for (k,v) in m.iter(){
        if skill_regex.is_match(k){
            let q = m.get(&format!("{}Mod",k)).expect("Mod not found");
            set(&v.as_str().unwrap().replace(" ",""),q.as_str().unwrap());
        }
    }
    let get_value = |v_name:&str| {m.get(v_name).unwrap().as_str().unwrap()};

    //Then do all of the attributes and values which are otherwise useful
    set("reflex",get_value("Reflex"));
    set("str",get_value("StrMod"));
    set("dex",get_value("DexMod"));
    set("con",get_value("ConMod"));
    set("int",get_value("IntMod"));
    set("wis",get_value("WisMod"));
    set("cha",get_value("ChaMod"));
    set("init",get_value("Init"));
    set("fort",get_value("Fort"));
    set("will",get_value("Will"));

    println!("finished porting");
}

//TODO
fn port_sf (user:&str,m:&serde_json::Map<String,serde_json::Value>){}


//sends the given string as a reply to the user, with a mention to them included
fn reply((author_mention,cid):(String,ChannelId),ctx:Context,s:&str) {
    let reply_time = Instant::now();
    let s = format!("{},\n{}",author_mention,s);
    match cid.say(ctx.http, s){
        Ok(_)=>(),
        Err(e)=>println!("{:?}",e),
    };

    println!("replying alone took {} ms",reply_time.elapsed().as_millis());
}

fn main() {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    if let Err(e) = client.start() {
        println!("Client error: {:?}", e);
    }
    println!("Started");
}