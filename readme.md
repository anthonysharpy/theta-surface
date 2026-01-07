# Title

## Usage

1. `cd` into the project's root directory.
2. Build with `cargo build`.
3. **(optional)** Run `./target/debug/ThetaSurface fetch-market-data` to download the latest market data. This is optional as the software is packaged with data by default. The data takes 5-10 minutes to download.
4. Run `./target/debug/ThetaSurface build-surface` to generate a volatility surface optimised to fit the downloaded data.