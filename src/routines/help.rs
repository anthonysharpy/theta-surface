pub fn help() {
    print!(
        "===== COMMANDS =====,

help:               Show this screen,
fetch-market-data:  Download the latest market data for analysis, saving the results in /data.
build-surface:      Build the volatility surface by analysing the downloaded data, saving the results in /data.
build-graphs:       Create graphs showing the implied volatility against strike price for each option expiry, saving the results in /data/graphs.
"
    )
}
