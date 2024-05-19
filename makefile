.DEFAULT_GOAL := frat

frat: ./src/*.rs
	cargo build --release
	mv ./target/release/frat-xor .

clean:
	rm frat-xor
