from lexrules import tokens, P
import math

tokens = tokens[:-3]
rpn = ""
semantic_error = False
zero_div = False
not_invertible = False
precedence = (
    ('left', 'ADD', 'SUB'),
    ('left', 'MUL', 'DIV'),
    ('right', 'NEG'),
    ('nonassoc', 'POW')
)

def p_line_expr(p):
    'line : expr ENDL'
    global rpn, zero_div, not_invertible, semantic_error
    print("rpn:", rpn)
    if semantic_error:
        if zero_div:
            print("Error: division by 0")
        elif not_invertible:
            print("Error: not invertible modulo 1234576")
        else:
            print("Error: invalid syntax")
    else:
        print(f"result = {p[1]}")
    rpn = ""
    semantic_error = False
    zero_div = False
    not_invertible = False


def p_line_error(p):
    'line : error ENDL'
    global rpn, zero_div, not_invertible, semantic_error
    if zero_div:
        print("Error: division by 0")
    elif not_invertible:
        print("Error: not invertible modulo 1234576")
    else:
        print("Error: invalid syntax")
    rpn = ""
    semantic_error = False
    zero_div = False
    not_invertible = False

def p_expr_number(p):
    'expr : number'
    # 'expr : NUM'
    global rpn
    rpn += f"{p[1] % P} "
    if not semantic_error:
        p[0] = p[1] % P

def p_expr_paren(p):
    'expr : LPAREN expr RPAREN'
    if not semantic_error:
        p[0] = p[2]

def p_expr_neg(p):
    'expr : SUB LPAREN expr RPAREN %prec NEG'
    global rpn
    rpn += "n "
    if not semantic_error:
        p[0] = (-p[3] + P) % P

def p_expr_add(p):
    'expr : expr ADD expr'
    global rpn
    rpn += "+ "
    if not semantic_error:
        p[0] = (p[1] + p[3]) % P

def p_expr_sub(p):
    'expr : expr SUB expr'
    global rpn
    rpn += "- "
    if not semantic_error:
        p[0] = (p[1] - p[3] + P) % P

def p_expr_mul(p):
    'expr : expr MUL expr'
    global rpn
    rpn += "* "
    if not semantic_error:
        p[0] = (p[1] * p[3]) % P

def p_expr_pow(p):
    'expr : expr POW exprPow'
    global rpn
    rpn += f"^ "
    if not semantic_error:
        p[0] = pow(p[1], p[3], P)

def p_expr_div(p):
    'expr : expr DIV expr'
    global rpn, zero_div, semantic_error
    rpn += "/ "
    if not semantic_error:
        if p[3] == 0:
            zero_div = True
            semantic_error = True
        else:
            p[0] = (p[1] * pow(p[3], -1, P)) % P

def p_number_pos(p):
    'number : NUM'
    p[0] = p[1]

def p_number_neg(p):
    'number : SUB number %prec NEG'
    p[0] = -p[2] % P



def p_exprPow_number(p):
    'exprPow : numberPow'
    # 'expr : NUM'
    global rpn
    rpn += f"{p[1]} "
    if not semantic_error:
        p[0] = p[1] % P

def p_exprPow_paren(p):
    'exprPow : LPAREN exprPow RPAREN'
    if not semantic_error:
        p[0] = p[2]

def p_exprPow_neg(p):
    'exprPow : SUB LPAREN exprPow RPAREN %prec NEG'
    global rpn
    rpn += "n "
    if not semantic_error:
        p[0] = (-p[3] + (P - 1)) % (P - 1)

def p_exprPow_add(p):
    'exprPow : exprPow ADD exprPow'
    global rpn
    rpn += "+ "
    if not semantic_error:
        p[0] = (p[1] + p[3]) % (P - 1)

def p_exprPow_sub(p):
    'exprPow : exprPow SUB exprPow'
    global rpn
    rpn += "- "
    if not semantic_error:
        p[0] = (p[1] - p[3] + (P - 1)) % (P - 1)

def p_exprPow_mul(p):
    'exprPow : exprPow MUL exprPow'
    global rpn
    rpn += "* "
    if not semantic_error:
        p[0] = (p[1] * p[3]) % (P - 1)

def p_exprPow_div(p):
    'exprPow : exprPow DIV exprPow'
    global rpn, zero_div, not_invertible, semantic_error
    rpn += "/ "
    if not semantic_error:
        if p[3] == 0:
            zero_div = True
            semantic_error = True
        else:
            gcd = math.gcd(p[1], p[3])
            p[1] //= gcd
            p[3] //= gcd
            if p[3] != 1 and (P - 1) % p[3] == 0:
                not_invertible = True
                semantic_error = True
            else:
                p[0] = (p[1] * pow(p[3], -1, (P - 1))) % (P - 1)

def p_numberPow_pos(p):
    'numberPow : NUM'
    p[0] = p[1]

def p_numberPow_neg(p):
    'numberPow : SUB numberPow %prec NEG'
    p[0] = -p[2] % (P - 1)

def p_error(p):
    pass
