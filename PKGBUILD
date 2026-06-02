# Maintainer: Carter <synthalorian@proton.me>
pkgname=reticulum-forge
pkgver=1.0.0
pkgrel=1
pkgdesc="CLI toolkit for building, testing, and deploying Reticulum mesh networks"
arch=('x86_64' 'aarch64')
url="https://github.com/synthalorian/reticulum-forge"
license=('Apache-2.0')
depends=()
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('be4216ce3488503428a6589ce20d4e00e4e92d5a2bd0103bc32f34fcf8a5e241')

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --locked
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --release --locked
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/forge" "$pkgdir/usr/bin/forge"
    install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Shell completions
    install -Dm644 "completions/forge.bash" "$pkgdir/usr/share/bash-completion/completions/forge"
    install -Dm644 "completions/_forge" "$pkgdir/usr/share/zsh/site-functions/_forge"
    install -Dm644 "completions/forge.fish" "$pkgdir/usr/share/fish/vendor_completions.d/forge.fish"

    # Man page
    install -Dm644 "man/forge.1" "$pkgdir/usr/share/man/man1/forge.1"
}
