%{
#include <stdio.h>
#include <string>

#define P 1234577

int yylex();
int yyparse();
void yyerror(char const* s);

int extended_euclid(int a, int b, int* x, int* y);

int zp(int a, int p);
int zp_add(int a, int b, int p);
int zp_sub(int a, int b, int p);
int zp_mul(long a, int b, int p);
int zp_div(long a, int b, int p);
int zp_inv(int a, int p);
int zp_pow(int a, int b, int p);

std::string rpn = "";
std::string error_message = "";

bool semantic_error = false;
%}

%token NUM
%token ERR
%left '+' '-'
%left '*' '/'
%precedence NEG
%right '^'

%%

input:
	 %empty
	 | input line
;

line:
	'\n'
	| exp '\n' {printf("rpn: %s\n", rpn.c_str()); rpn = ""; if (semantic_error) {printf("error: %s\n", error_message.c_str()); semantic_error = false; error_message = "";} else printf("result: %d\n", $1);}
	| error '\n' {if (error_message == "") printf("error: invalid syntax\n"); else printf("%s\nerror: %s\n", rpn.c_str(), error_message.c_str()); rpn = ""; error_message = "";}
;

exp:
	number {rpn += std::to_string($1) + " "; if (!semantic_error) $$ = $1;}
	| exp '+' exp {rpn += "+ "; if (!semantic_error) $$ = zp_add($1, $3, P);}
	| exp '-' exp {rpn += "- "; if (!semantic_error) $$ = zp_sub($1, $3, P);}
	| exp '*' exp {rpn += "* "; if (!semantic_error) $$ = zp_mul($1, $3, P);}
	| exp '/' exp {rpn += "/ "; if (!semantic_error) {if ($3 == 0) {error_message = "division by 0"; semantic_error = true;} else {$$ = zp_div($1, $3, P);}}}
	| '-' '(' exp ')' %prec NEG   {rpn += "n "; if (!semantic_error) $$ = zp(-1 * $3, P);}
	| exp '^' exppow {rpn += "^ "; if (!semantic_error) $$ = zp_pow($1, $3, P);}
	| '(' exp ')' {if (!semantic_error) $$ = $2;}
;

number:
	NUM {$$ = zp($1, P);}
	| '-' number %prec NEG {$$ = zp(-1 * $2, P);}
;

exppow:
	numberpow {rpn += std::to_string($1) + " "; if (!semantic_error) $$ = $1;}
	| exppow '+' exppow {rpn += "+ "; if (!semantic_error) $$ = zp_add($1, $3, P - 1);}
	| exppow '-' exppow {rpn += "- "; if (!semantic_error) $$ = zp_sub($1, $3, P - 1);}
	| exppow '*' exppow {rpn += "* "; if (!semantic_error) $$ = zp_mul($1, $3, P - 1);}
	| exppow '/' exppow {rpn += "/ "; if (!semantic_error) {if ($3 == 0) {error_message = "division by 0"; semantic_error = true;} else {int tempx; int tempy; int gcd = extended_euclid($1, $3, &tempx, &tempy); $1 /= gcd; $3 /= gcd; if ($3 != 1 && (P - 1) % $3 == 0) {error_message = "not invertible mod 1234576"; semantic_error = true;} else {$$ = zp_div($1, $3, P - 1);}}}}
	| '-' '(' exppow ')' %prec NEG {rpn += "n "; if (!semantic_error) $$ = zp(-1 * $3, P - 1);}
	| '(' exppow ')' {if (!semantic_error) $$ = $2;}
;

numberpow:
	NUM {$$ = zp($1, P - 1);}
	| '-' numberpow %prec NEG {$$ = zp(-1 * $2, P - 1);}
;

%%

int extended_euclid(int a, int b, int* x, int* y) {
	if (a == 0) {
		*x = 0;
		*y = 1;
		return b;
	}
	int x1, y1;
	int d = extended_euclid(b % a, a, &x1, &y1);
	*x = y1 - (b/a) * x1;
	*y = x1;
	return d;
}

int zp(int a, int p) {
	while (a < 0) {
		a += p;
	}
	while (a >= p) {
		a -= p;
	}
	return a;
}

int zp_add(int a, int b, int p) {
	return (a + b) % p;
}

int zp_sub(int a, int b, int p) {
	return zp_add(a, zp(-1 * b, p), p);
}

int zp_mul(long a, int b, int p) {
	return (a * b) % p;
}

int zp_div(long a, int b, int p) {
	return zp_mul(a, zp_inv(b, p), p);
}

int zp_inv(int a, int p) {
	int x, y;
	extended_euclid(a, p, &x, &y);
	return zp(x, p);
}

int zp_pow(int a, int b, int p) {
	if (b == 0) {
		return 1;
	}
	long c = zp_pow(a, b / 2, p);
	if (b % 2 == 0) {
		return (c * c) % p;
	} else {
		return (a * ((c * c) % p)) % p;
	}
}

void yyerror(char const* s) {
	return;
}

int main() {
	yyparse();
	return 0;
}
