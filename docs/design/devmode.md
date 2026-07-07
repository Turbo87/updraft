# Developer Mode

Like Android's hidden developer options, Updraft ships a developer mode in
production builds: tapping the version number seven times unlocks
additional debugging options that are otherwise invisible.

Options unlocked by developer mode include:

- the byte-capture replay transport (see [devices.md](devices.md)), which
  replays recorded raw device bytes through the real framer, dispatcher,
  and parsers
- map rendering debugging
- data loading debugging

The list grows as debugging needs appear. The bar for adding an option here
is much lower than for a user-facing setting.
