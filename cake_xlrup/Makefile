# Default build for x64
cake_xlrup: basis_ffi.c cake_xlrup.S
	gcc -O2 basis_ffi.c cake_xlrup.S -o cake_xlrup -std=c99

clean:
	rm -fv cake_xlrup
