build:
	cargo build --release

install:
	cp ./target/release/pleco /usr/bin/

uninstall:
	rm /usr/bin/pleco
