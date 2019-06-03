const Discord = require('discord.js');
const config = require("./config.json");
const request = require('request');
const util = require('util');
var ohm = require('ohm-js');
var fs = require('fs');
const client = new Discord.Client();
var nestvarcount=0;
var repeatRecursion=0;
var unwrapped='';
var defurled='';
var contents = fs.readFileSync('grammar.ohm');
var g = ohm.grammar(contents);
var semantics = g.createSemantics();

semantics.addOperation('eval', {
  Exp: function(e) {
    return e.eval();
  },
  TextExp: function(e) {
    return this.sourceString;
  },
  AddExp: function(e) {
    return e.eval();
  },
  AddExp_plus: function(left, op, right) {
    console.log("pre"+unwrapped);
    var l=left.eval();
    unwrapped+="+";
    console.log("mid"+unwrapped);
    defurled+="+";
    var r=right.eval();
    console.log("post"+unwrapped);
    return l+r;
  },
  AddExp_minus: function(left, op, right) {
    var l=left.eval();
    defurled+="-";
    unwrapped+="-";
    var r=right.eval();
    return l-r;
  },
  AddExp_strApp: function(str,op,exp) {
    var s=str.eval();
    var r=exp.eval();
    return s+r;
  },
  MulExp_times: function(left, op, right) {
    var l=left.eval();
    defurled+="*";
    unwrapped+="*";
    var r=right.eval();
    return l*r;
  },
  MulExp_divide: function(left, op, right) {
    var l=left.eval();
    defurled+="/";
    unwrapped+="/";
    var r=right.eval();
    return l/r;
  },
  ExpExp_power: function(left, op, right) {
    var l=left.eval();
    defurled+="^";
    unwrapped+="^";
    var r=right.eval();
    return Math.pow(l,r);
  },
  PriExp: function(e) {
    return e.eval();
  },
  PriExp_paren: function(open, exp, close) {
    return exp.eval();
  },
  RepeatRoll: function(r, oparen,ex,comma, rnum, cparen) {
    if (this.repeatrecursion>100)return 1;
    this.repeatrecursion++;
    rnum=rnum.eval();
    var s = [];

    defurled+="(";
    unwrapped+="(";
    for (var x =0;x<rnum;x++){
      if (x!=0){ defurled+=","; unwrapped+=",";}

      s[x]=ex.eval();
    }
    defurled+=")";
    unwrapped+=")";
    return s;
  },
  FrontEq_frontEq: function(vari, eq, exp) {
    var variv=vari.eval();
    var expv = exp.eval();
    console.log("inner eq with "+expv);
    var user = cmdList[currentUser];
    user.varSet(variv,expv);
    var match = g.match(expv);//run matching on the source text
    return semantics(match).eval();
  },
  BackEq_backEq: function(exp, eq, vari) {
    var variv=vari.eval();
    var expv = exp.eval();
    console.log("inner eq with "+expv);

    var user = cmdList[currentUser];
    user.varSet(variv,expv);
    return expv;
    //var match = g.match(expv);//run matching on the source text
    //return semantics(match).eval();
  },
  String: function(oquot,txt,equot) {
    var s = this.sourceString;
    var o="";
    for (var i =1;i<s.length-1;i++){
      if (s[i]==="\\" &&s[i+1]=="n"){
        o+="\n";
        i+=1;
      }

      else o+=s[i];
    }

    defurled+='"'+o+'"';
    unwrapped+='"'+o+'"';
    return o;
  },
  Var: function(dollar, ident) {
    if (this.nestvarcount>40)return 1;
    this.nestvarcount++;

    var user = cmdList[currentUser];
    var vRes = user.varAc(this.sourceString)
    console.log(`defurling ${this.sourceString}, unwrapped value:${vRes}`);
    if ((typeof vRes)==="undefined") return;
    var match = g.match(vRes);
    return semantics(match).eval();
  },
  VarEq: function(term) {
    console.log(`sourcing ${this.sourceString}`);
    return this.sourceString;
  },
  Roll_norm: function(l, d, r) {
    l=l.eval();
    r=r.eval();
    unwrapped+=`${l}d${r}`;

    var s=0;
    defurled+="("
    for (var x =0;x<l;x++){
      var i =Math.floor((Math.random()*r) +1);
      s+=i;
      if (x!=0){
        defurled+="+";
      }
      defurled+=i;
    }

    defurled+=")"

    return s;
  },
  Roll_shortnorm: function(d, r) {
    var rAmt=r.eval();
    var val=Math.floor((Math.random()*rAmt) +1);
    defurled+=val
    unwrapped+=`d${rAmt}`;
    return val;
  },
  Roll_keep: function(l, d, m,k,r){
    var rollval=l.eval();
    var mval=m.eval();
    var keepval=r.eval();
    unwrapped+=`${rollval}d${mval}k${keepval}`;
    if (keepval>rollval)return;
    var s=[];
    for (var x =0;x<rollval;x++){

      s[x]=Math.floor((Math.random()*mval) +1);
    }
    console.log(s);
    s=s.sort().reverse();

    function strikeThrough(text) {
      return text.split('').map(char => '\u0336'+char ).join('')
    }

    defurled+="(";
    for (var x =0;x<rollval;x++){
      if (x!=0){defurled+="+";}
      var str = ""+s[x];
      if (x>=keepval)defurled+=strikeThrough(str);
      else {defurled+=str;}
    }

    defurled+=")";

    s=s.slice(0,keepval);
    console.log(s);
    function getSum(total, num) {
      return total + num;
    }

    var out =s.reduce(getSum);
    //  defurled+=" = "+out+" ";
    return out;
  },
  LoneNumber: function(chars) {//A number not part of a comp exp (i.e. 3 instead of 3d6). We want to keep these in defurled for debugging purposes
    var v = chars.eval()
    defurled+=v;
    unwrapped+=v;
    return v;
  },
  Number_whole: function(chars) {
    var v = parseInt(this.sourceString, 10);
    return v;
  },
  Number_fract: function(l,p,r) {
    var v = parseFloat(this.sourceString);
    return v;
  }
});
semantics.addOperation('text',{Exp:function(e){return this.sourceString}});


var currentUser;
const mwStart = "https://www.myth-weavers.com/api/v1/sheets/sheets/";

class Player{
  constructor(v,cc){
    //console.log("constructing ");
    this.vars=v;
    this.currentChar=cc;
  }
  newChar(name){
    name = name.toLowerCase();
    this.vars[name]={};
    this.currentChar=name;
    saveState();
  }

  varSet(vName,value){
    vName=vName.toLowerCase().trim();
    if (typeof (this.vars[this.currentChar])==="undefined") {(this.vars[this.currentChar])={};}
    this.vars[this.currentChar][vName] = value;
    saveState();
  }
  list(message){
    if ((typeof this.vars[this.currentChar])==="undefined" || size_dict(this.vars[this.currentChar])==0) {
      (this.vars[this.currentChar])={};
      message.channel.send("You have no defined terms");
      return;
    }
    //else
    message.channel.send(`Here are your defined terms ${Object.keys(this.vars[this.currentChar])}`);
  }

  varRemove(name){
    name=name.toLowerCase().trim();
    delete this.vars[this.currentChar][name];
    saveState();
  }

  varAc(name){
    name = name.toLowerCase().trim();
    if (typeof (this.vars[this.currentChar])==="undefined") {(this.vars[this.currentChar])={};}
    if (typeof (this.vars[this.currentChar][name])==="undefined"){
      console.log("badAccess");
      throw "You attempted to access an invalid element";
    }
    return this.vars[this.currentChar][name];
  }
}

var cmdList = {};

client.on('ready', () => {
  console.log(`Logged in as ${client.user.tag}!`);
});

client.on('error', () => {
  console.log(`Error`);
});

client.on('message', (message) => {

  try{
    var author = message.author
    var messageText=message.content.trim();;
    if (!messageText.startsWith(config.prefix) || author.bot) return;

    currentUser=message.author.id;
    var user = cmdList[currentUser];
    console.log("user is player " + user instanceof Player);
    if (typeof(user) === "undefined") {user = cmdList[currentUser] = new Player({},undefined);}

    const args = messageText.trim().split(/ +/g);
    const cmd = args.shift().toLowerCase().substring(1);

    if (cmd==="port") {
      var cn = args.shift();
      var valid = new RegExp("^[0-9]+$").test(cn);
      if (!valid) {message.channel.send("Please provide the mythweavers sheet ID number");return;}

      request(mwStart+cn,function (error, response, html) {
        if (error)console.error("req failed",error);
        var sd = JSON.parse(html).sheetdata;
        try{

          var jData = JSON.parse(sd.sheet_data.jsondata);
          jKeys = Object.keys(jData);

          var name = sd["name"];
          console.log(`Porting new char ${name}`);
          console.log(`user=${user}`);
          user.newChar(name);
          user.varSet("$character",'"'+name+'"');
          ///devolve any responsibility of specific character attributes beyond name to game-specific handlers
          switch (sd.sheet_template_id) {
            case 11://3.5
            console.log("importing a 3.5 sheet");
            handle_35e(user,jData,jKeys);
            break;
            case 43://starfinder
            console.log("importing an sf sheet");
            handle_sf(user,jData,jKeys);
            break;
          }

          user.list(message);
        }
        catch(err){
          console.log(err);
          message.channel.send("Sorry, it seems that the id is invalid. Maybe your sheet is private?");
        }
      });

    } else if (cmd==="char"){
      var name = args.join(' ');
      user.newChar(name);
      message.channel.send("Created a character named "+name);
    }else if (cmd==="h"||cmd==="help"){
      message.channel.send(`
        ![h/help] you should know what this does
        ![listchar/listchars] will list the valid characters you have defined
        ![char] [name] will create a blank character
        ![delchar] [name] will remove said character
        ![switch] [name] switch to the given characters
        ![port] [id] will port an entire mythweavers sheet (or at least the important stuff) given the number at the end of the url

        ![a/assign] $[varname] [expression] will assign a variable. Note that variables *can* be used in expressions
        ![l/list] lists your personal arguments
        ![rename] $[varname] $[varname] deletes the first var and puts its value in the second
        ![r/roll] [expression] will return the result of that roll
        ![re/rollexplicit] [expression] will act like a roll, but tell you the intermediate values
        ![v/value] $[varname] will print what the roll system thinks of a variable as
        ![c/clear] $[varname] will delete a variable
        ![a/assign] $[varname] [expression] will assign the variable to that expression (and recompute that expression every time the variable is resolved)
        ![ic/incharacter] [text] will make this bot print the text in a way that shows your character said it. If you have a discord emote which has the same name as your $character variable, it will include that icon.
        Valid expressions are combinations of basic Arithmetic operations, numbers, rolls (e.g. 3d6), and variables
        Cool roll expressions include $[varname],  [x]d[y]k[z] (e.g. 4d6k3), the repeat block r([expression],[number]) which will quit after your call invokes more than 100 repeats, and inline var assignment (e.g $x=d20+2 or $x=$y). If a variable takes more than 40 variable resolves before it itself resolves, any further nested variables are treated as 1
        Note: the expression "$x = d20" will reroll that value each time $x is used, but "d20 = $x" will store the result of that roll and not recalculate. They can also be used in larger expressions (e.g. "d20 + $health = d6)."`);
      }
      else if (cmd==="listchar"||cmd==="listchars"||cmd==="charlist"){
        if (size_dict(user.vars) ==0)  message.channel.send("You don't seem to have any characters");
        else {
          var out = "You have the following characters defined:";
          for (char in user.vars){
            out+=`\n${char}`;
          }
          if (!typeof(user.currentChar) === "undefined"){out+=`\nYou are currently ${user.currentChar}`;}
          message.channel.send(out);
        }
      }

      else if (typeof(user.currentChar) === "undefined") {message.channel.send("It seems that you're trying to use the bot without any defined characters! Port one from MW with 'port' or create a blank one with 'char'");return;}

      else if (cmd==="switch"){
        var name = args.join(' ').toLowerCase();
        if (name in user.vars) {
          user.currentChar = name;
          message.channel.send("Switched to "+name);
        }else{
          message.channel.send("That character doesn't exist");
        }

      } else if (cmd==="delchar"){
        var name = args.join(' ').toLowerCase();
        if (name in user.vars) {
          delete user.vars[name];
          saveState();
          if (size_dict(user.vars) ==0 || user.currentChar == name) {delete user.currentChar; message.channel.send("Successfully deleted! Please create or switch to a valid character");}
          else message.channel.send("Successfully deleted that other character");
        }else {message.channel.send("That Character doesn't seem to exist");}

      } else if (cmd==="rename"){
        var orig=args.shift();
        var ne=args.shift();
        if (!orig.startsWith("$")||!ne.startsWith("$")) {message.channel.send("Please start both varnames with $"); return;}

        user.varSet(ne, user.varAc(orig));
        user.varRemove(orig);
        message.channel.send("Updated");
      } else if (cmd==="l"||cmd==="list"){
        user.list(message);
      } else if (cmd==="i"||cmd==="ic"||cmd==="incharacter"){
        message.delete(10);//delete the msg with 10ms delay

        try{
          var match =g.match("$character");
          if (match.failed()) throw Error;
          var out = semantics(match).eval();
          console.log("out= "+ out);
          var outemoji = client.emojis.find(emoji => emoji.name === out);
          console.log("outem"+outemoji);
          if (outemoji===null) outemoji="💩";
          console.log(`emoji type ${typeof outemoji} is null ${outemoji===null}`);
          if ((typeof outemoji)==="undefined"){throw "Use Alias";}
        }
        catch(e){out=message.member.nickname;outemoji="💩";console.log("missing emoji!");console.log(e);}

        try{
          var color;
          var cmatch =g.match("$color");
          if (cmatch.succeeded()) {color = semantics(cmatch).eval();}
        }
        catch(e){color="RANDOM";}

        var richEm = new Discord.RichEmbed();
        richEm.addField(`${outemoji} (${out}) says:`,messageText.slice(messageText.indexOf(" ")),true);
        richEm.setColor(color.toUpperCase());
        message.channel.send("",{embed:richEm});

      } else if (cmd==="value"||cmd==="v"){
        var v = args.shift();
        message.channel.send(v+" : "+user.varAc(v));
      } else if (cmd==="c"||cmd==="clear"){
        var next = args.shift()
        if (typeof(next)!="undefined"){
          next=next.toLowerCase();
          if (next in user.vars[user.currentChar]){
            user.varRemove(next);
            message.channel.send(`Your variable ${next} has been cleared`)
            saveState();
            return;
          }
          else if (!(next==="confirm")){message.channel.send("If you're trying to clear me, type confirm after the clear");return;}
          user.vars[user.currentChar]={};
          message.channel.send("Your (and only your) commands have been cleared");
          saveState();
        }
        else {message.channel.send("If you're trying to clear me, type confirm after the clear");}
      }else if (cmd==="r"||cmd==="roll"){
        var exp=args.join(' ');
        var out = roll(exp);
        message.channel.send(out,
          {
            code:true,
            reply:currentUser
          }
        );
      }
      else if (cmd==="re"||cmd==="rollexplicit"){
        var exp=args.join(' ');
        var [uw,df,out] = roll_explicit(exp);

        message.channel.send(`(${uw.trim()}) => (${df.trim()}) = ${out}`,
        {
          code:true,
          reply:currentUser
        }
      );
    } else if (messageText.startsWith("a")||messageText.startsWith("assign")){
      var v=args.shift();
      if (! (new RegExp("[a-z]+").test(v))){message.channel.send("start your varnames with $, only containing letters"); return; }
      var exp=args.join(' ');

      var match =g.match(exp);
      if (match.succeeded()) {user.varSet(v,exp); message.channel.send("Assigned!"); }
      else message.channel.send("Invaild expression");
    }
  }

  catch(e){
    console.log(e);
  }
}
);
client.login(config.token);
loadState();
//console.log(cmdList);

function saveState(){
  var fs = require('fs');
  var util = require('util');
  var jstring = JSON.stringify(cmdListUnwrap());
  fs.writeFileSync('cmdList.json', jstring, 'utf-8')
}
function loadState(){
  try{
    const cl= fs.readFileSync('cmdList.json','utf-8');

    rewrapcmdList( JSON.parse(cl));

  }catch(e){console.log("error in state load");console.log(e);}
}
function roll(exp){
  unWrapped='';
  defurled='';
  nestvarcount=0;
  repeatRecursion=0;
  var match =g.match(exp);
  console.log("ohmMatch: "+match);
  if (!match.succeeded()) return "Grammar Failure";
  try{
    console.log("testing match");
    console.assert(match.succeeded());
    console.log("Match Succeeded");
    return semantics(match).eval();
  }
  catch(e){return e;}
  //if exp has an equality, it gets handled by varSet
}
function roll_explicit(exp){
  var out = roll(exp);
  return [unwrapped,defurled,out];
}

///changes 3 to "+3" and -2 to "-2"
function intfmt(int){
  return (jData["Init"]>=0?"+":"")+jData["Init"];
}

function handle_35e(player, jData,jKeys){
  for (var x=0;x<jKeys.length;x++){
    let key = jKeys[x];

    if (new RegExp("^Skill([0-9]{2})$").test(key)){
      player.varSet("$"+(jData[key]).replace(/\s/g,""),`1d20${intfmt(jData[key+"Mod"])}`);
    }
  }
  player.varSet("$reflex",`1d20${intfmt(jData["Reflex"])}`);
  player.varSet("$str",`1d20${intfmt(jData["StrMod"])}`);
  player.varSet("$dex",`1d20${intfmt(jData["DexMod"])}`);
  player.varSet("$con",`1d20${intfmt(jData["ConMod"])}`);
  player.varSet("$int",`1d20${intfmt(jData["IntMod"])}`);
  player.varSet("$wis",`1d20${intfmt(jData["WisMod"])}`);
  player.varSet("$cha",`1d20${intfmt(jData["ChaMod"])}`);
  player.varSet("$init",`1d20${intfmt(jData["Init"])}`);
  player.varSet("$fort",`1d20${intfmt(jData["Fort"])}`);
  player.varSet("$will",`1d20${intfmt(jData["Will"])}`);
}

function handle_sf(player,jData,jKeys){
  for (var x=0;x<jKeys.length;x++){
    let key = jKeys[x];
    if (new RegExp("^skill_([0-9]+)_name$").test(key)){
      var sn = key.substring(0,key.length-5);//without _name
      player.varSet("$"+(jData[key]).replace(/\s/g,""),
      `1d20${intfmt(jData[sn+"_skill_mod"])}`);
    }
  }

  player.varSet("$fort",`1d20${intfmt(jData["fortitude_total"])}`);
  player.varSet("$con",`1d20${intfmt(jData["Constitution_Mod"])}`);
  player.varSet("$int",`1d20${intfmt(jData["Intelligence_Mod"])}`);
  player.varSet("$reflex",`1d20${intfmt(jData["reflex_total"])}`);
  player.varSet("$dex",`1d20${intfmt(jData["Dexterity_Mod"])}`);
  player.varSet("$cha",`1d20${intfmt(jData["Charisma_Mod"])}`);
  player.varSet("$str",`1d20${intfmt(jData["Strength_Mod"])}`);
  player.varSet("$init",`1d20${intfmt(jData["Init_total"])}`);
  player.varSet("$will",`1d20${intfmt(jData["will_total"])}`);
  player.varSet("$wis",`1d20${intfmt(jData["Wisdom_Mod"])}`);
}

function size_dict(d){c=0; for (i in d) ++c; return c}

function cmdListUnwrap(){
  //console.log(cmdList);
  var full = {}
  for (var uid in cmdList){
    var us = {}
    //console.log("[uid] "+cmdList[uid]);
    us["currentChar"]=cmdList[uid].currentChar;
    us["vars"]=cmdList[uid].vars;
    full[uid]=us;
  }
  //console.log(full);
  return full;
}
function rewrapcmdList(list){
  //console.log("reoconstructing from ");
  //console.log(list);
  var full = {}
  for (var uid in list){
    //console.log("doing "+uid);
    var va = list[uid]["vars"];
    //console.log(va);
    var cc = list[uid]["currentChar"];
    //console.log(cc);
    var p = new Player(va,cc);
    //console.log("passed");
    full[uid]=p;
  }
  //console.log("reconstructed");
  //console.log(full);
  cmdList=full;

}
