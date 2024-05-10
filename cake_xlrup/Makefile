# Default build for x64
cake_xlrup: basis_ffi.c cake_xlrup.S
	gcc basis_ffi.c cake_xlrup.S -o cake_xlrup -static -std=c99 \
		-DCML_HEAP_SIZE=4096 -DCML_STACK_SIZE=4096

clean:
	rm -fv cake_xlrup
