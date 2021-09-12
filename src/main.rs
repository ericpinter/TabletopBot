#![feature(result_flattening)]
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
    model::{channel::Message, gateway::Ready, id::ChannelId},
    prelude::*,
    framework::standard::macros::*,
    framework::standard::*,
};
use std::time::Instant;
use serenity::http::CacheHttp;
use std::collections::HashSet;
use serenity::model::id::UserId;
use crate::parser::is_valid;

#[command]
#[num_args(1)]
///Takes the myth-weavers id associated with a character sheet. This is the number at the end of the url when viewing the sheet.
///This command then ports the skills, and attributes associated with the given sheet into a character in this bot.
///Currently only 3.5 and starfinder sheets are supported, and not all information is ported.
fn port(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let id = args.current().unwrap();
    let user = msg.author.id.0.to_string();
    let response = port_character(&user, &id);
    reply(ctx, msg, &response.unwrap_or_else(|s| s))
}

#[command]
#[min_args(1)]
///Takes a name and creates an empty character with that name.
fn char(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    let name = args.rest();
    add_char(&user, &name);
    reply(ctx, msg, &format!("Created a char named: {} ", name))
}

#[command]
#[aliases(lc, lchar, listchars, charlist)]
///Lists the characters that you have defined.
fn listchar(ctx: &mut Context, msg: &Message, _args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    reply(ctx, msg, &list_chars(user).ok_or("Command failed")?)
}

#[command]
#[aliases(setcc)]
#[min_args(1)]
///Takes a name and attempts to switch to that character.
fn switch(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    reply(ctx, msg, &set_cc(&user, &args.rest()).ok_or("Command failed")?)
}


#[command]
#[aliases(del)]
#[min_args(1)]
///Takes a name and attempts to delete that character.
fn delchar(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    reply(ctx, msg, &remove_char(&user, &args.rest()).ok_or("Command failed")?)
}

//TODO help command?


#[command]
#[aliases(r)]
#[min_args(1)]
///Takes an expression and evaluates it.
///Addition, Subtraction, Multiplication, Division, Exponentiation and Parenthesis may be used, as well as dice rolls in the forms d20, 3d6, and 4d6k3.
///Variables in the form $x are evaluated as stand-ins (use !assign to give them specific values)
///For example we might define (!assign $strength 12), and then (!assign $attack d6 + $strength). At this point we could then simply roll $attack when necessary.
///The repeat expression may be used to repeat a command a fixed number of times. For example repeat($attack,6).
///Text expressions may also be used, but must be surrounded by quotes.
///Two equality operators are defined. ($x = d30) would mean on each subsequent use of $x a new 30 sided die would be rolled. (d30 = $x) would store the result of a single roll and always use that value.
///The ternary operator (t ? [expression 1] : [expression 2]) returns the value [expression 1] if t is not zero. If it is zero it returns the value [expression 2]
///Just for added fun, variables can be referenced indirectly. For example storing "b" in $a and evaluating $($a) is equivalent to $b. This also works with ternary statements
fn roll(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    let out = {
        match parse(user, args.rest()) {
            Ok(cal) => cal.output.to_string(),
            Err(e) => format!("ERROR!: {}", e)
        }
    };
    reply(ctx, msg, &out)
}


#[command]
#[aliases(re)]
#[min_args(1)]
///Takes an expression and evaluates it, showing the steps of variable resolution and evaluation.
fn roll_explicit(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    let out = match parse(user, args.rest()) {
        Ok(parse_result) => {
            format!("({}) → ({}) → {}", parse_result.defurled, parse_result.unwrapped, parse_result.output)
        }
        Err(_) => { String::from("Invalid Input") }
    };
    reply(ctx, msg, &out)
}

#[group]
#[commands(port, char, listchar, switch, delchar, roll, roll_explicit)]
struct General;


#[check]
#[name(valid_current_character)]
fn valid_current_character(_: &mut Context, msg: &Message, _: &mut Args, _: &CommandOptions) -> CheckResult {
    let user = msg.author.id.0.to_string();
    if valid_cc(&user) { CheckResult::Success } else { CheckResult::Failure(Reason::User(String::from("It seems you do not have a valid current CharacterBased. Use the !char command to make a new one or !switch to switch to one you already have."))) }
}

#[command]
#[aliases(a)]
#[min_args(2)]
///Takes the name of a variable (prefixed with $) and a valid expression. The expression is then stored in the variable.
///Note that the expression is recalculated each time the variable is used, so !a $x [expr] is equivalent to !r $x = [expr].
fn assign(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    let var = args.current().unwrap().to_owned();

    let response = if regex::Regex::new(r"\$[[a-zA-Z]\d_()]").unwrap().is_match(&var) {
        args.advance();
        let exp = args.rest();
        if is_valid(&exp) {
            set_var(&user, &var, &exp);
            "Assigned!"
        } else {
            "The given expression was invalid"
        }
    } else { "Please start all vars with $, and use only a-z A-Z _ 0-9 and () in the variable's name" };
    reply(ctx, msg, response)
}


#[command]
#[aliases(l)]
///Lists the variables you have defined in the current character
fn list(ctx: &mut Context, msg: &Message, _args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    reply(ctx, msg, &list_vars(&user).ok_or("Vars not found")?)
}


#[command]
#[min_args(1)]
#[aliases(v)]
///Takes the name of a variable (prefixed with $). Returns the raw (un-evaluated) value associated with that variable.
fn value(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    //TODO be wary of unwrapping and resolution before
    let name = args.rest();
    let val = match resolve(&user, name) {
        Some(s) => { s }
        None => { String::from("Does not exist!") }
    };

    reply(ctx, msg, &format!("{} : {}", name, val))
}


#[command]
#[min_args(2)]
///Takes two variable names (both prefixed with $). Reassigns the value in the first variable into the second, deleting the first.
fn rename(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    let prev = args.current().unwrap().to_owned();
    args.advance();
    let next = args.rest();

    let response = match resolve(&user, &prev) {
        Some(val) => {
            set_var(&user, &next, &val);
            remove_var(&user, &prev);
            format!("Renamed {} to {}", prev, next)
        }
        None => {
            format!("{} does not seem to exist", prev)
        }
    };
    reply(ctx, msg, &response)
}


#[command]
#[aliases(ic)]
#[only_in(guilds)]
#[min_args(1)]
///Takes a message and displays it in a pretty embedded message.
///Defining the value $color (with a hexadecimal color code) lets you change the color on the left of the embed.
///The variable $character_emoji can assigned to the name of an emoji. The icon in the embedd will then be that emoji.
///Note that because these variables are a part of the roll evaluation system they should not be surrounded by quotes, unlike other text.
fn inchar(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let user = msg.author.id.0.to_string();
    let out = match resolve(&user, "$character_emoji") {
        Some(s) => { s }
        None => { msg.author.name.clone() }
    };

    let color =
        if let Some(s) = resolve(&user, "$color") {
            u64::from_str_radix(&s, 16)?
        } else { 123456 };

    //actually guaranteed by the only_in guilds flag
    if let Some(guild) = msg.guild(&ctx.cache) {
        let icon_url =
            match guild.read().emojis.values().find(|e| { e.name == out }) {
                //try and find a custom emoji named after their CharacterBased
                Some(icon) => { icon.url() }
                None => {
                    //otherwise try to just make it their avatar
                    msg.author.avatar_url().unwrap_or(
                        //otherwise give them something universal
                        String::from("https://modworkshop.net/mydownloads/previews/preview_54895_1540694735_b03cf8b0fc082142d5ab1ff8a7dc0fb4.jpg"))
                }
            };

        let text = args.rest();
        let result = msg.channel_id.send_message(&ctx, |m| {
            m.embed(|e| {
                e.title(format!("{}", text))
                    .author(|aut| aut.name(&out).icon_url(icon_url))
                    .color(color)
            })
        });
        println!("{:?}", result);
        msg.delete(&ctx)?;
    }

    Ok(())
}

#[group]
#[commands(assign, list, value, rename, inchar)]
#[checks(valid_current_character)]
struct CharacterBased;

struct Handler;

impl EventHandler for Handler {}


#[help]
fn help(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

fn port_character(user: &str, num: &str) -> Result<String, String> {
    if regex::Regex::new(r"^[0-9]+$").expect("Regex failed").is_match(num) {
        let url = format!("https://www.myth-weavers.com/api/v1/sheets/sheets/{}", num);
        let response = reqwest::blocking::get(&url).ok().ok_or("Request failed")?;
        let json: serde_json::Value = response.json().ok().ok_or("Request worked, but the given sheet has no json")?;
        //Mythweavers formats their data like this, don't blame me for the mess
        let sheet_template = json["sheetdata"]["sheet_template_id"].as_u64().ok_or("Failed to find template")?;

        let map_str = json["sheetdata"]["sheet_data"]["jsondata"].as_str().ok_or("Failed to traverse json")?;
        let map_val: serde_json::Value = serde_json::from_str(map_str).ok().ok_or("failed to parse nested json")?;
        let map = map_val.as_object().ok_or("Map")?;

        let char_name = map.get("Name").expect("Name not found").as_str().ok_or("finding char_name")?;
        println!("Making a CharacterBased named {}", char_name);
        add_char(user, char_name);

        match sheet_template {
            11 => {
                port_35e(user, map)?;
                list_vars(user).ok_or("Failed to summarize character".into())
            }
            43 => {
                port_sf(user, map);
                list_vars(user).ok_or("Failed to summarize character".into())
            }
            other => {
                println!("Failed to port sheet of type {:?}", other);
                Err(format!("Are you sure that this is a sheet of the right type? Type number {} may not be supported at the moment", other))
            }
        }
    } else { Err(String::from("Please make sure you have only the number at the end of your mw sheet in this command")) }
}

fn port_35e(user: &str, m: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    println!("Starting the port!");
    let set = |k: &str, v: &str| set_var(user, &format!("${}", k), &format!("d20{}{}", if v.parse::<i64>().unwrap() >= 0 { "+" } else { "" }, v));

    let skill_regex = regex::Regex::new("^Skill[0-9]{2}$").unwrap();
    //port over all skills
    for (k, v) in m.iter() {
        if skill_regex.is_match(k) {
            let q = m.get(&format!("{}Mod", k)).ok_or("Mod not found")?;
            set(&v.as_str().ok_or("Type error")?.replace(" ", "").replace("(", "-").replace(")", ""), q.as_str().ok_or("Type error")?);
        }
    }
    let get_value = |v_name: &str| { m.get(v_name).ok_or("Missing Value")?.as_str().ok_or("Type error") };

    //Then do all of the attributes and values which are otherwise useful
    set("reflex", get_value("Reflex")?);
    set("str", get_value("StrMod")?);
    set("dex", get_value("DexMod")?);
    set("con", get_value("ConMod")?);
    set("int", get_value("IntMod")?);
    set("wis", get_value("WisMod")?);
    set("cha", get_value("ChaMod")?);
    set("init", get_value("Init")?);
    set("fort", get_value("Fort")?);
    set("will", get_value("Will")?);

    println!("finished porting");
    Ok(())
}

//TODO
fn port_sf(_user: &str, _m: &serde_json::Map<String, serde_json::Value>) {}


//sends the given string as a reply to the user, with a mention to them included
//TODO handle overlong messages more gracefully
fn reply(ctx: &mut Context, msg: &Message, s: &str) -> CommandResult {
    let reply_time = Instant::now();
    let s = format!("{},\n{}", msg.author.mention(), s);
    msg.channel_id.say(ctx.http(), s)?;

    println!("replying alone took {} ms", reply_time.elapsed().as_millis());
    Ok(())
}

fn main() {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("!")
            .case_insensitivity(true))
        .group(&GENERAL_GROUP).group(&CHARACTERBASED_GROUP)
        .on_dispatch_error(|ctx, msg, err| {
            reply(ctx, msg, &format!("Error {:?} while executing that command", err));
        })
        .unrecognised_command(|ctx, msg, given_cmd| {
            reply(ctx, msg, &format!("The command {} was unrecognized", given_cmd));
        })
        .help(&HELP)
    );

    if let Err(e) = client.start() {
        println!("Client error: {:?}", e);
    }
    println!("Started");
}