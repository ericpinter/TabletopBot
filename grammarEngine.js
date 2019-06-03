var ohm = require('ohm-js');
var fs = require('fs');
var contents = fs.readFileSync('grammar.ohm');

class Engine{
  constructor(){
    this.defurled = "";
    this.unwrapped = "";
    this._g = ohm.grammar(contents);
    this._semantics = this._g.createSemantics();


    this._semantics.addOperation('eval', {
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
        console.log("pre"+this.unwrapped);
        var l=left.eval();
        this.unwrapped+="+";
        console.log("mid"+this.unwrapped);
        this.defurled+="+";
        var r=right.eval();
        console.log("post"+this.unwrapped);
        return l+r;
      },
      AddExp_minus: function(left, op, right) {
        var l=left.eval();
        this.defurled+="-";
        this.unwrapped+="-";
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
        this.defurled+="*";
        this.unwrapped+="*";
        var r=right.eval();
        return l*r;
      },
      MulExp_divide: function(left, op, right) {
        var l=left.eval();
        this.defurled+="/";
        this.unwrapped+="/";
        var r=right.eval();
        return l/r;
      },
      ExpExp_power: function(left, op, right) {
        var l=left.eval();
        this.defurled+="^";
        this.unwrapped+="^";
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

        this.defurled+="(";
        this.unwrapped+="(";
        for (var x =0;x<rnum;x++){
          if (x!=0){ this.defurled+=","; this.unwrapped+=",";}

          s[x]=ex.eval();
        }
        this.defurled+=")";
        this.unwrapped+=")";
        return s;
      },
      FrontEq_frontEq: function(vari, eq, exp) {
        var variv=vari.eval();
        var expv = exp.eval();
        console.log("inner eq with "+expv);
        var user = this.cmdList[this.currentUser];
        user.varSet(variv,expv);
        var match = this._g.match(expv);//run matching on the source text
        return this._semantics(match).eval();
      },
      BackEq_backEq: function(exp, eq, vari) {
        var variv=vari.eval();
        var expv = exp.eval();
        console.log("inner eq with "+expv);

        var user = this.cmdList[this.currentUser];
        user.varSet(variv,expv);
        return expv;
        //var match = this._g.match(expv);//run matching on the source text
        //return this._semantics(match).eval();
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

        this.defurled+='"'+o+'"';
        this.unwrapped+='"'+o+'"';
        return o;
      },
      Var: function(dollar, ident) {
        if (this.nestvarcount>40)return 1;
        this.nestvarcount++;

        var user = this.cmdList[this.currentUser];
        var vRes = user.varAc(this.sourceString)
        console.log("defurling "+this.sourceString+", this.unwrapped value:"+vRes);
        if ((typeof vRes)==="undefined") return;
        var match = this._g.match(vRes);
        return this._semantics(match).eval();
      },
      VarEq: function(term) {
        console.log("sourceing "+this.sourceString);
        return this.sourceString;
      },
      Roll_norm: function(l, d, r) {
        l=l.eval();
        r=r.eval();
        this.unwrapped+=""+l+"d"+r;

        var s=0;
        this.defurled+="("
        for (var x =0;x<l;x++){
          var i =Math.floor((Math.random()*r) +1);
          s+=i;
          if (x!=0){
            this.defurled+="+";
          }
          this.defurled+=i;
        }

        this.defurled+=")"
        //this.defurled+=" = "+s+" ";

        return s;
      },
      Roll_shortnorm: function(d, r) {
        var rAmt=r.eval();
        var val=Math.floor((Math.random()*rAmt) +1);
        this.defurled+=val
        this.unwrapped+="d"+rAmt;
        return val;
      },
      Roll_keep: function(l, d, m,k,r){
        var rollval=l.eval();
        var mval=m.eval();
        var keepval=r.eval();
        this.unwrapped+=rollval+"d"+mval+"k"+keepval;
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

        this.defurled+="(";
        for (var x =0;x<rollval;x++){
          if (x!=0){this.defurled+="+";}
          var str = ""+s[x];
          if (x>=keepval)this.defurled+=strikeThrough(str);
          else {this.defurled+=str;}
        }

        this.defurled+=")";

        s=s.slice(0,keepval);
        console.log(s);
        function getSum(total, num) {
          return total + num;
        }

        var out =s.reduce(getSum);
        //  this.defurled+=" = "+out+" ";
        return out;
      },
      LoneNumber: function(chars) {//A number not part of a comp exp (i.e. 3 instead of 3d6). We want to keep these in this.defurled for debugging purposes
        var v = chars.eval()
        this.defurled+=v;
        this.unwrapped+=v;
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
    this._semantics.addOperation('text',{Exp:function(e){return this.sourceString}});

  }

  set g(grammar){
    this._g=grammar;
  }

  get g(){
    return this._g;
  }
  get semantics(){
    return this._semantics;
  }
}

module.exports=new Engine();
