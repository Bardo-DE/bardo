//! The shipped list of network programs: what it makes of the commands people run.

use permission_core::{NetPrograms, net_reach};

fn shipped() -> NetPrograms {
    toml::from_str(include_str!("../../../dist/network-programs.toml"))
        .expect("the shipped list reads")
}

#[test]
fn the_shipped_list_sorts_the_commands() {
    let (yes, no) = (true, false);
    let cases = [
        ("curl https://a.test/x", yes),
        ("/usr/bin/curl -s a.test", yes),
        ("wget a.test", yes),
        ("ssh host ls", yes),
        ("scp a host:b", yes),
        ("rsync -a . host:/b", yes),
        ("nc -l 9", yes),
        ("git push origin main", yes),
        ("git -C /home/u/proj push", yes),
        ("git fetch", yes),
        ("git clone x", yes),
        ("git pull", yes),
        ("git status", no),
        ("git diff HEAD~1", no),
        ("git log --oneline", no),
        ("cargo publish", yes),
        ("cargo install ripgrep", yes),
        ("cargo build", no),
        ("cargo test --lib", no),
        ("cargo clippy", no),
        ("npm publish", yes),
        ("npm install left-pad", yes),
        ("npm test", no),
        ("npm run build", no),
        ("pip install x", yes),
        ("pip list", no),
        ("ls -la", no),
        ("cat Cargo.toml", no),
        ("echo hello", no),
        ("cd src", no),
        ("pwd", no),
        ("sh -c 'curl a.test'", yes),
        ("env FOO=1 wget a.test", yes),
        ("echo hi; curl a.test", yes),
        ("ls | nc host 9", yes),
        ("ls && cargo test", no),
        ("cat curl.txt", no),
        ("echo $(ssh h)", yes),
    ];
    let programs = shipped();
    for (line, want) in cases {
        assert_eq!(net_reach(line, false, &programs), want, "{line}");
    }
}
