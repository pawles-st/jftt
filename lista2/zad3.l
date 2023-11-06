%top{
	#include <stdio.h>
}

%{
	int docmode = 0;
%}

%option noyywrap

COMMENT1        "//"[^/!][^\n]*\n
COMMENT2        "/*"[^*!](\**[^*/]+\/*)*"*/"
DOCCOMMENT1     "///"[^\n]*\n
DOCCOMMENT2     "//!"[^\n]*\n
DOCCOMMENT3     "/**"(\**[^*/]+\/*)*"*/"
DOCCOMMENT4     "/*!"(\**[^*/]+\/*)*"*/"
STRING          \"(([^\\\"])*(\\[\\bnrtv\"])*)*\"
INCLUDE_PATH    \<[^\>\n]*\>
INCLUDE         #include[ \t]*({STRING}|{INCLUDE_PATH})

%s DOCMODE

%%
	if (docmode) {
		BEGIN(DOCMODE);
	}

{COMMENT1}                    ;
^[ \t]*{COMMENT1}             ;
{COMMENT2}                    ;
^[ \t]*{COMMENT2}[ \t]*\n     ;
<DOCMODE>{DOCCOMMENT1}        ECHO;

{INCLUDE}                     ECHO;
{STRING}                      ECHO;

<INITIAL>{
^[ \t]*{DOCCOMMENT1}          ;
{DOCCOMMENT1}                 ;
^[ \t]*{DOCCOMMENT2}          ;
{DOCCOMMENT2}                 ;
^[ \t]*{DOCCOMMENT3}[ \t]*\n  ;
{DOCCOMMENT3}                 ;
^[ \t]*{DOCCOMMENT4}[ \t]*\n  ;
{DOCCOMMENT4}                 ;
}

.|\n                       ECHO;

%%

int main(int argc, char **argv) {
	if (argc > 1 && strcmp(argv[1], "doc") == 0) {
		docmode = 1;
	}
	yylex();
	return 0;
}
