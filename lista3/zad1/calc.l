%option noyywrap

%{
#include <stdio.h>
#include <stdlib.h>
#include "calc.hpp"
#define P 1234577

extern int yylex();
%}


%%
[ \t]  ;
^#(.|\\\n)*\n ;
\\\n   ;
[[:digit:]]+ { 
    yylval = atoi(yytext); 
    return NUM; 
}
"(" {return '(';}
")" {return ')';}
"+" {return '+';}
"-" {return '-';}
"*" {return '*';}
"/" {return '/';}
"^" {return '^';}
\n  {return '\n';}
.   {return ERR;}
%%
