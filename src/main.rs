use chrono::Local;
use clap::Parser as CliParser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger::Builder;
use log::info;
use poise::serenity_prelude as serenity;
use regex::Regex;
use reqwest::{Client, Response};
use std::io::Write;
use tabled::settings::Style;
use url::{ParseError, Url};

struct Data {
    url: String,
    output_limit: String,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(CliParser, Debug)]
#[command(version, about, long_about = None)]
struct CLI {
    #[arg(long, default_value = "https://clickhouse.nxthdr.dev")]
    url: String,

    /// ClickHouse user
    #[arg(short, long)]
    user: String,

    /// ClickHouse password
    #[arg(short, long)]
    password: String,

    /// Discord bot token
    #[arg(short, long)]
    token: String,

    /// Max output lines
    #[arg(long, default_value = "10")]
    output_limit: String,

    /// Verbosity level
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn set_logging(cli: &CLI) {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter_module("chbot", cli.verbose.log_level_filter())
        .init();
}

async fn format_url(cli: &CLI) -> Result<String, ParseError> {
    let url = Url::parse(&cli.url)?;
    let qs = format!("?user={}&password={}", cli.user, cli.password);
    Ok(url.join(&qs)?.to_string())
}

async fn format_query(query: String, output_limit: i32) -> Result<String, Error> {
    let mut formatted_query = query.clone();

    let re = Regex::new(r".*LIMIT\s(\d+).*$").unwrap();
    let limit: Option<i32> = match re.captures(&query) {
        Some(caps) => Some(caps.get(1).unwrap().as_str().parse().unwrap()),
        None => None,
    };
    if let Some(limit) = limit {
        if limit > output_limit {
            formatted_query = query.replace(
                &format!("LIMIT {}", limit),
                &format!("LIMIT {}", output_limit),
            );
        }
    } else {
        formatted_query = format!("{} LIMIT {}", query, output_limit)
    }

    let re = Regex::new(r".*FORMAT\s(\S+).*$").unwrap();
    let format: Option<String> = match re.captures(&formatted_query) {
        Some(caps) => Some(caps.get(1).unwrap().as_str().to_string()),
        None => None,
    };
    if let Some(_) = format {
        return Err("Please don't put any FORMAT".into());
    } else {
        formatted_query = format!("{} FORMAT CSVWithNames", formatted_query)
    }

    Ok(formatted_query)
}

async fn do_query(query: String, url: String) -> Result<Response, Error> {
    let client = Client::new();
    let time_start = std::time::Instant::now();
    let resp = client.post(url).body(query.clone()).send().await?;
    let time_end = std::time::Instant::now();
    let time_diff = time_end - time_start;
    info!("`{}` took {:?}", query, time_diff);
    return Ok(resp);
}

async fn pretty_print(text: String) -> String {
    let table = csv_to_table::from_reader(text.as_bytes())
        .unwrap()
        .with(Style::sharp())
        .to_string();

    // Return the table in a code block
    // This will make it look nice in Discord
    return format!("```{}```", table);
}

#[poise::command(slash_command, prefix_command)]
async fn query(
    ctx: Context<'_>,
    #[description = "Query"] query_text: Option<String>,
) -> Result<(), Error> {
    let query_text = match query_text {
        Some(query_text) => query_text,
        None => {
            ctx.say("Please provide a query").await?;
            return Ok(());
        }
    };

    let output_limit: i32 = ctx.data().output_limit.clone().parse().unwrap();
    let query_text = match format_query(query_text, output_limit).await {
        Ok(query_text) => query_text,
        Err(e) => {
            ctx.say(format!("{}", e)).await?;
            return Ok(());
        }
    };
    let resp = do_query(query_text, ctx.data().url.clone()).await?;
    let text = pretty_print(resp.text().await?).await;

    ctx.say(text).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = CLI::parse();
    set_logging(&cli);

    let url = format_url(&cli).await?;
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![query()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    url,
                    output_limit: cli.output_limit,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(cli.token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_format_query() {
        assert_eq!(
            format_query("SELECT Count() FROM nxthdr.bgp_updates".to_string(), 10)
                .await
                .unwrap(),
            "SELECT Count() FROM nxthdr.bgp_updates LIMIT 10 FORMAT CSVWithNames".to_string()
        );

        assert_eq!(
            format_query(
                "SELECT Count() FROM nxthdr.bgp_updates LIMIT 5".to_string(),
                10
            )
            .await
            .unwrap(),
            "SELECT Count() FROM nxthdr.bgp_updates LIMIT 5 FORMAT CSVWithNames".to_string()
        );

        assert_eq!(
            format_query(
                "SELECT Count() FROM nxthdr.bgp_updates LIMIT 50".to_string(),
                10
            )
            .await
            .unwrap(),
            "SELECT Count() FROM nxthdr.bgp_updates LIMIT 10 FORMAT CSVWithNames".to_string()
        );

        assert!(format_query(
            "SELECT Count() FROM nxthdr.bgp_updates FORMAT Pretty".to_string(),
            10
        )
        .await
        .is_err());
    }
}
