# Title

## Usage

1. `cd` into the project's root directory.
2. Build with `cargo build`.
3. **(quasi-optional)** Run `./target/debug/ThetaSurface fetch-market-data` to download the latest market data. This is sort-of optional as the software is packaged with data by default. However, since expired options are discarded, if you don't download fresh data then you might end up with none of the data being unuseable. The data takes around 5-10 minutes to download.
4. Run `./target/debug/ThetaSurface build-surface` to generate a volatility surface optimised to fit the downloaded data.