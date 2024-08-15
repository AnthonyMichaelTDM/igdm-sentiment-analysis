# Instagram DM Sentiment Analysis

Runs sentiment analysis on instagram DM's and plots them over time.

## Dependencies

- `anyhow` - easy error handling
- `clap` - cli args (probably overkill)
- `plotters` - rendering the "sentiment over time" plots
- `serde` and `serde_json` - json parsing
- `vader_sentiment` - sentiment analysis using rust port of the VADER algorithm
- `walkdir` - util for resursively walking directories

## Usage

1. install rust, clone the repo, and compile the executable
2. export your instagram data (you can look up how to do this), make sure you select json as the formatting option for messages.
3. run the executable, and pass it the path to the directory containing the messages for the chat you wish to analyze (for example: `meta-2024-<etc>/your_instagram_activity/messages/inbox/instagramuser_1962803592016810/`).


