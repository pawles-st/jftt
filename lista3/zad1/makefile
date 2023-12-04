main: calc.y calc.l
	bison -d -o calc.cpp calc.y
	flex -o calc.c calc.l
	g++ -o calc calc.cpp calc.c

clean:
	rm calc.cpp calc.hpp calc.c calc
