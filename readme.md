# Theta Surface

Rust tool for fetching Bitcoin market option data, fitting a volatility surface and generating graphs showing the implied volatility against strike price.

The purpose of this is to demonstrate the implementation of complex market-based mathematics and algorithms in a clear and structured way. It is not necessarily meant to be the fastest or most efficient implementation. For an example of fast low-latency programming, see https://github.com/anthonysharpy/nanofill.

# Graph explanation

![Graph](./images/example-smile-graph.png)

- **Extrapolated data**: The extrapolated part of the smile curve.
- **Observed data**: The smile curve representing the range of data that we actually observed.
- **Smile-relative implied volatility**: The implied volatility of an option acccording to the fitted smile curve.
- **Self-relative implied volatility**: The actual implied volatility of an option according to its own data. Basically, this is what the curve is trying to fit. Multiple of these on the same strike usually implies a put/call pair. If these show a trend that differs greatly from the curve, it implies that the curve was badly fit.

## Usage

1. Enter the project's root directory.

```
cd theta-surface
```

2. Build the project.

```
cargo build
```

3. Download the latest market data. This is semi-optional as the software is packaged with data by default. However, since expired options are discarded, if you don't download fresh data then none of the included data might be useable. It may also affect the quality of the output. The data takes 5-10 minutes to download and is saved in `/data`.

```
./target/debug/ThetaSurface fetch-market-data
```

4. Fit the volatility surface for the downloaded data. This data is also saved in `/data`. Fitting takes 1-2 minutes on a middle-of-the-range laptop.

```
./target/debug/ThetaSurface build-surface
```

5. Generate graphs showing the implied volatility against strike price for each option expiry. These are saved to `/data/graphs` as .png files.

```
./target/debug/ThetaSurface build-graphs
```

## How it works

_**fetch-market-data**_

- Bitcoin option data is downloaded from Deribit's public cryptocurrency API. An option is a contract granting the right to buy or sell an asset (in this case Bitcoin) at a pre-determined price (**strike price**) on a pre-determined date (**expiry**).
- For consistency, we normalise all downloaded data to have the same spot price.
- This data is saved to file.

_**build-surface**_

- The download Deribit data is loaded from file.
- This data is converted into a simpler internal format. Any invalid options are discarded (e.g. options that have already expired).
- These options are then grouped by expiry. Typically, there will be a wide range of options with different strike prices for the same expiry.
- A smile graph is constructed for each group. The smile graph shows how the (implied) volatility of the option changes as the strike price changes, which typically looks like a smile.
- When using the smile graph, we must determine a single forward price for the underlying (Bitcoin) per smile. Since we already normalised spot prices, they are all the same, so we just pick the first one. For consistency, we plug this into the same forward-price formula that we use for solving implied volatility.
- Creating the smile graph ("fitting") involves using a guessing-based algorithm to find the most accurate curve that fits the data. In this case the Levenberg-Marquardt algorithm is used. The curve we fit is based on the SVI formula, which is designed to usually produce curves that are valid according to conventional enonomic theory (but not always, so we also manually check for arbitrage).
- Checks for valid bounds and butterfly arbitrage etc. are carried out during fitting in order to ensure an (economically) mathematically valid fit. In order to arrive at the best fit, we brute force starting guesses within reasonable ranges derived from the data. We also use a patience-based method, where the algorithm makes faster leaps when it enters areas of no improvement.
- Under the hood, the use of the SVI formula actually produces a graph showing how total implied variance changes as log moneyness changes. This is not actually what we're interested in, but it's required to make the math work. We'll convert this back later.
- The curves for each group are saved to file, as well as some other information about the smile and the options belonging to it that will help us when building the graphs later.

_**build-graphs**_

- The smile graphs for each expiry group are loaded from file.
- Any existing graphs are deleted.
- A graph is constructed for each smile. The first and last quarter of each graph is extrapolated data. The middle half of each graph is in the range of the data that we actually observed from the API, and so is likely the most accurate.
- In creating the graphs some math is performed to convert the smiles from a graph showing the change in total implied variance changes against log moneyness into a graph that shows how implied volatility changes as the strike price changes.
- Once constructed, these graphs are saved to file.
- These graphs are then useful for things such as efficiently pricing options at any given strike price, accurately analysing the market when other data is noisy, validating that other market data is accurate and tracking changes in risk and uncertainty over time.

## Known limitations

- Fitting can be slow. I think there are still optimisations to be made here. An obvious one would be making it multi-threaded.
- Poor-quality data (e.g. options with weird prices) is not removed, which can negatively affect the overall fit.
- More tests are needed. The basic mathematical pieces like the implied volatility calculations have tests, but there is a lack of tests in other places. This project already took a long time to put together, and I just don't fancy spending days writing tests for it all. Sorry. Thankfully though the program is mostly self-testing since it displays most things on the graph, which can be checked manually.