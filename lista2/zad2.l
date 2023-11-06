%top {
	#include <stdio.h>
}

%{
char *comment_start; 
%}

COMMENT "<!--"(\-?[^\-]+)*"-->"
STRING \"(([^\"\\])*(\\[\\vftnb\"])*)*\"

%option noyywrap

%x CDATA TAG

%%

^[ \t]*{COMMENT}[ \t]*\n
{COMMENT}
\<                        ECHO; BEGIN(TAG);
<TAG>{STRING}             ECHO;
<TAG>\>                   ECHO; BEGIN(INITIAL);

%%

int main() {
	yylex();
	return 0;
}
