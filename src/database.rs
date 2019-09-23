//db is a hashmap of hashmaps [Users] -> [Commands] -> (Str value of cmd)

//perhaps I should switch to a real fast-hashmap implementation, but I had fun making this and until this code is actually serving hundreds of users a second (it won't ever be) it isn't worth it.
use std::collections::HashMap;
use std::fmt::Error;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};

const SAVE_LOC: &str = "./cmdList.json";

lazy_static! {
    pub static ref USER_MAP:RwLock<UserMapStruct>={load_db()};
}

pub type UserMapStruct = HashMap<String, User>;//UserName -> (map of char names to (map of eqs.))

#[bench]
pub fn stress_test(b: &mut test::Bencher) {
    b.iter(|| stress(100));
}

pub fn stress(i: u32) {
    add_char("test", "test");
    for _ in 0..i {
        std::thread::spawn(||{
        set_var("test", "$x", "$x");
        let result = match resolve("test", "$x") {
            Some(r) => { r }
            None => String::from("FUCKED UP"),
        };
        remove_var("test", &result);
        });
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User { current_char: String, vars: HashMap<String, Character> }

// name -> (map of var name to eqs.)
impl User {
    //insert a new char
    fn insert(&mut self, n: String, c: Character) {
        self.vars.insert(n, c);
    }
}

pub type Character = HashMap<String, String>;//varName -> eq.

fn load_db() -> RwLock<UserMapStruct> {
    let new = UserMapStruct::new();
    let blank =
        RwLock::new(UserMapStruct::new());

    match std::fs::read_to_string(SAVE_LOC) {
        Ok(s) => {
            let js: Result<UserMapStruct, Error> = serde_json::from_str(&s).or(Ok(new));
            match js {
                Ok(m) => {
                    RwLock::new(m)
                }
                Err(e) => {
                    println!("{:?}", e);
                    blank
                }
            }
        }
        Err(e) => {
            println!("{:?}", e);
            blank
        }
    }
}

///BE CAREFUL TO ONLY CALL THIS WHEN YOU DON'T OWN USER_MAP's LOCK!
pub fn save_db() {

    std::thread::spawn(move || {

        let lock = USER_MAP.read().unwrap();
        if let Ok(stringify) = serde_json::to_string(&*lock) {
            drop(lock);

            let result = std::fs::write(SAVE_LOC, &stringify);
            println!("{:?}",result);
        };
    });
}

pub fn list_vars(user: &str) -> String {
    let lock = USER_MAP.read().unwrap();
    let user = (*lock).get(user).unwrap();
    let ch = user.vars.get(&user.current_char).unwrap();

    if ch.len() == 0 {
        String::from("You've got no defined variables")
    } else {
        let mut keys = ch.keys();
        let mut s = keys.next().unwrap().to_string();//len>0 means safe unwrap
        for k in keys { s += &format!(", {}", k); }
        s
    }
}

pub fn list_chars(user: String) -> String {
    let lock = USER_MAP.read().unwrap();
    let user = (*lock).get(&user).unwrap();

    if user.vars.len() == 0 {
        String::from("You've got no defined characters")
    } else {
        let mut keys = user.vars.keys();
        let mut s = keys.next().unwrap().to_string();//len>0 means safe unwrap
        for k in keys { s += &format!(", {}", k); }
        s + &format!("\n    Current Character: {}", user.current_char)
    }
}

pub fn set_cc(user: &str, cc: &str) -> String {
    let mut lock = USER_MAP.write().unwrap();
    let user = (*lock).get_mut(user).unwrap();

    if user.vars.contains_key(cc) {
        user.current_char = cc.to_string();
        drop(lock);
        save_db();
        format!("Switched to {}", cc)
    } else { String::from("That Character doesn't seem to exist") }

}

pub fn add_char(user: &str, name: &str) {
    let name = &name.to_lowercase();
    {
        let user = user;
        let name = name.clone();
        let mut map = USER_MAP.write().expect("failed to get map addChar");
        match (*map).get_mut(user) {
            Some(this_user) => {
                println!("Updating Old User");
                this_user.insert(name, Character::new());
            }
            _ => {
                println!("Making new User");
                let mut blank = User { current_char: String::from(""), vars: HashMap::new() };
                let char = Character::new();
                blank.insert(name, char);
                (*map).insert(user.to_string(), blank);
            }
        };
    }
    set_cc(user, &name);
    set_var(user, "$character", &name);
    save_db();
}

pub fn remove_char(user: &str, name: &str) -> String {
    let name = name.to_lowercase();
    let mut lock = USER_MAP.write().unwrap();
    let user = (*lock).get_mut(user).unwrap();
    let remove: bool = user.vars.contains_key(&name);
    let switch_necessary: bool = user.current_char.eq(&name);
    let r = if remove {
        user.vars.remove(&name);
        if switch_necessary {
            String::from("Removed character. Please switch to a valid one")
        } else {
            String::from("Removed that other character.")
        }
    } else { String::from("That character doesn't seem to exist") };
    drop(lock);
    save_db();
    r
}

pub fn resolve(user: &str, v_name: &str) -> Option<String> {
    let v_name = v_name.to_lowercase();
    let lock = USER_MAP.read().unwrap();
    let user = (*lock).get(user).unwrap();
    let ch = user.vars.get(&user.current_char).unwrap();
    //println!("ch => {:?}", ch);
    //println!("user => {:?}", user);

    if ch.contains_key(&v_name) {
        Some(ch.get(&v_name).unwrap().to_string())
    } else { None }
}

pub fn set_var(user: &str, v_name: &str, value: &str) {
    //std::thread::spawn(move||{
    let mut lock = USER_MAP.write().expect("failed to lock");
    let v_name = v_name.to_lowercase();

    let user = (*lock).get_mut(user).unwrap();
    let ch = user.vars.get_mut(&user.current_char).unwrap();
    ch.insert(v_name.to_string(), value.to_string());
    drop(lock);
    save_db();
    //});
}

pub fn remove_var(user: &str, v_name: &str) -> String {
    let v_name = v_name.to_lowercase();
    let mut lock = USER_MAP.write().unwrap();
    let user = (*lock).get_mut(user).unwrap();
    let ch = user.vars.get_mut(&user.current_char).unwrap();
    let r = if ch.contains_key(&v_name) {
        ch.remove(&v_name);
        String::from(format!("Your variable {} has been cleared", v_name))
    } else {
        if v_name.eq("confirm") {
            user.insert(user.current_char.clone(), Character::new());
            String::from("Your (and only your) commands have been cleared")
        } else {
            String::from("If you're trying to clear me, type confirm after the clear")
        }
    };
    drop(lock);
    save_db();
    r
}

pub fn valid_cc(u_name: &str) -> bool {
    let lock = USER_MAP.read().unwrap();

    match (*lock).get(u_name) {
        Some(user) => user.vars.contains_key(&user.current_char),
        None => false,
    }
}