%top {
	#include <stdio.h>
}

%{
int no_lines = 0;
int no_words = 0;
%}

%%

^[ \t]+
^[ \t]+\n       ++no_lines;
[ \t]+$
[ \t]+          putchar(' ');
[^ \t\n]+       ECHO; ++no_words;
\n              ECHO; ++no_lines;

%%

int yywrap() {
	printf("no. lines = %d\n", no_lines);
	printf("no. words = %d\n", no_words);
	return 1;
}

int main() {
	yylex();
	return 0;
}
