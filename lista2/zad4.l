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

%x ERROR
%s CALC

%%


<ERROR>[^\n]      ;
<ERROR>\n         ECHO; BEGIN(INITIAL);

<INITIAL>^[^\n]+\n ECHO; yyless(0); BEGIN(CALC);
<INITIAL>^\n      ;

[ \t]*            ;

{NUMBER}          {
	*stackptr = atoi(yytext);
	stackptr += 1;
}

{OPERATOR}        {
	if (stackptr == stack || stackptr == stack + 1) {
		stackptr = stack;
		printf("error: not enough arguments");
		BEGIN(ERROR);
	} else {
		b = *--stackptr;
		a = *--stackptr;
		REJECT;
	}
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
	if (b == 0) {
		stackptr = stack;
		printf("error: division by 0");
		BEGIN(ERROR);
	} else {
		*stackptr++ = a / b;
	}
}

"^"               {
	*stackptr++ = pow(a, b);
}

"%"               {
	if (b == 0) {
		stackptr = stack;
		printf("error: division by 0");
		BEGIN(ERROR);
	} else {
		*stackptr++ = a % b;
	}
}

<<EOF>>           {
	if (stackptr != stack) {
		printf("= %d\n", *--stackptr);
	}
	yyterminate();
}

.                 {
	stackptr = stack;
	printf("error: incorrect symbol \"%s\"", yytext);
	BEGIN(ERROR);
}

<CALC>\n          {
	if (stackptr == stack + 1) {
		printf("= %d\n", *--stackptr);
	} else {
		printf("error: not enough operators\n");
	}
	stackptr = stack;
	BEGIN(INITIAL);
}

%%

int main() {
	stackptr = stack;

	yylex();

	return 0;
}
