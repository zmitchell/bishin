# bishin

`bishin` is a totally rad way to run tests with shells.

Use cases:
- Ensure that your CLI application works well when run in multiple shells.
- Test that your shell code works well in multiple shells.

### Why does it exist
I originally wrote this to test [Flox](https://github.com/flox/flox),
where we've used `bats` since before I joined.
For context, Flox provides reproducible, subshell-based developer environments
that allow you to keep your dotfiles, etc (unlike containers).
As part of this we need to ensure that we handle shells in the correct way
on all the shells that we support (bash, zsh, fish, and tcsh at the moment).

There is a small number of other shell testing frameworks,
but none of them had the combination of features I was looking for,
namely:
- Support tests written for shells other than Bash at all
- Parameterize tests against the shell running the test
- Parameterize tests with other data
- Isolate the test environment by default (e.g. `$HOME` and XDG dirs).
- Record `stdout`, `stderr`, and `stdout+stderr` streams at the same time
- Inspect the environment of subcommands
- Provide excellent reporting out of the box

## FAQ
- Where did the name come from?
    - `bishin` is like _bitchin'_ but with more `sh`
    - If your boss starts to ask questions, just tell them it means "beautiful mind" in Japanese. That's what ChatGPT told me when I asked what `bishin` meant in other languages.
- Is this well tested?
    - Not yet
- Are PRs welcome?
    - Sure! Just comment on an issue if you'd like to pitch in.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
