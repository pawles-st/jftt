%top{
	#include <stdio.h>
	#include <math.h>

	#define STACK_SIZE 16
}

%{
	int stack[STACK_SIZE];
	int* stackptr;

	int a;
	int b;
%}

%option noyywrap

DIGIT             [0-9]
NUMBER            \-?{DIGIT}+
OPERATOR          [+\-*/^%]

%%

[ \t]*            ;

{NUMBER}          {
	*stackptr++ = atoi(yytext);
}

{OPERATOR}        {
	b = *--stackptr;
	a = *--stackptr;
	REJECT;
}

"+"               {
	*stackptr++ = a + b;
}

"-"               {
	*stackptr++ = a - b;
}

"*"               {
	*stackptr++ = a * b;
}

"/"               {
	*stackptr++ = a / b;
}

"^"               {
	*stackptr++ = pow(a, b);
}

"%"               {
	*stackptr++ = a % b;
}

<<EOF>>           {
	if (stackptr != stack) {
		printf("= %d\n", *--stackptr);
	}
	yyterminate();
}

.                 {
	printf("incorrect symbol: %s\n", yytext);
}

\n                {
	if (stackptr != stack) {
		printf("= %d\n", *--stackptr);
	}
	stackptr = stack;
}

%%

int main() {
	stackptr = stack;

	yylex();

	return 0;
}
