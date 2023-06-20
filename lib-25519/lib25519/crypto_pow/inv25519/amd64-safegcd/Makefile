CC=clang -O3 -march=native -Wall

test: test.o asm.o table.o
	$(CC) -o test test.o asm.o table.o -lgmp

test.o: test.c
	$(CC) -c test.c

asm.o: asm.s
	$(CC) -c asm.s

table.o: table.c
	$(CC) -c table.c
