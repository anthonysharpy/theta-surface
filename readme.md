# ThetaSurface

Tool for fetching Bitcoin market option data, fitting a volatility surface and generating graphs showing the implied volatility against strike price.

The purpose of this tool is to demonstrate the implementation of complex market-based mathematics and algorithms in a clear and structured way. It is not meant to be the fastest or most efficient implementation by any means. For an example of fast low-latency programming, see https://github.com/anthonysharpy/nanofill.

## Usage

1. Enter the project's root directory.

```
cd theta-surface
```

2. Build the project.

```
cargo build
```

3. Download the latest market data. This is semi-optional as the software is packaged with data by default. However, since expired options are discarded, if you don't download fresh data then none of the included data might be useable. The data takes 5-10 minutes to download and is saved in `/data`.

```
./target/debug/ThetaSurface fetch-market-data
```

4. Fit the volatility surface for the downloaded data. This data is also saved in `/data`.

```
./target/debug/ThetaSurface build-surface
```

5. Generate graphs showing the implied volatility against strike price for each option expiry. These are saved to `/data/graphs` as .png files.

```
./target/debug/ThetaSurface build-graphs
```

## How it works

_**fetch-market-data**_

- Bitcoin option data is downloaded from Deribit's public stock market data API. An option is a contract granting the right to buy a commodity (in this case Bitcoin) at a pre-determined price (**strike price**) on a pre-determined date (**expiry**).
- This data is saved to file.

_**build-surface**_

- The download Deribit data is loaded from file.
- This data is converted into a simpler internal format. Any invalid options are discarded (e.g. options that have already expired).
- These options are then grouped by expiry. Typically, there will be a wide range of options with different strike prices for the same expiry.
- A smile graph is constructed for each group. The smile graph shows how the (implied) volatility of the option changes as the strike price changes, which typically looks like a smile.
- Creating the smile graph ("fitting") involves using a guessing-based algorithm to find the most accurate curve that fits the data. In this case the Levenberg-Marquardt algorithm is used. The curve we fit is based on the SVI formula, which is designed so as to not produce curves that are impossible (according to conventional enonomic theory).
- Under the hood, the use of the SVI formula actually produces a graph showing how total implied variance changes as log moneyness changes. This is not actually what we're interested in, but it's required to make the math work. We'll convert this back later.
- The curves for each group are saved to file, as well as some other information about the smile and the options belonging to it that will help us when building the graphs later.

_**build-graphs**_

- The smile graphs for each expiry group are loaded from file.
- Any existing graphs are deleted.
- A graph is constructed for each smile. The first and last quarter of each graph is extrapolated data. The middle half of each graph is in the range of the data that we actually observed from the API, and so is likely the most accurate.
- In creating the graphs some math is performed to convert the smiles from a graph showing the change in total implied variance changes against log moneyness into a graph that shows how implied volatility changes as the strike price changes.
- Once constructed these graphs are saved to file.
- These graphs are then useful for things such as efficiently pricing options at any given strike price, accurately analysing the market when other data is noisy, validating that other market data is accurate and tracking changes in risk and uncertainty over time. 

# AI disclaimer

ChatGPT was used in this project for research and looking-up information. **None** of the code or text in this project is AI-generated.