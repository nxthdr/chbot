use chrono::Local;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger::Builder;
use log::info;
use poise::serenity_prelude as serenity;
use reqwest::{Client, Response};
use std::io::Write;
use tabled::settings::Style;

struct Data {
    url: String,
    output_limit: String,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Parser, Debug)]
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

async fn format_url(cli: &CLI) -> String {
    return format!("{}?user={}&password={}", cli.url, cli.user, cli.password);
}

async fn format_query(query: String, output_limit: String) -> String {
    return format!("{} LIMIT {} FORMAT CSVWithNames", query, output_limit);
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

    let query_text = format_query(query_text, ctx.data().output_limit.clone()).await;
    let resp = do_query(query_text, ctx.data().url.clone()).await?;
    let text = pretty_print(resp.text().await?).await;

    ctx.say(text).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = CLI::parse();
    set_logging(&cli);

    let url = format_url(&cli).await;
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
