use agcodex_arg0::arg0_dispatch_or_else;
use agcodex_common::CliConfigOverrides;
use agcodex_tui::Cli;
use agcodex_tui::run_main;
use clap::Parser;

#[derive(Parser, Debug)]
struct TopCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    inner: Cli,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        let top_cli = TopCli::parse();
        let mut inner = top_cli.inner;
        inner
            .config_overrides
            .raw_overrides
            .splice(0..0, top_cli.config_overrides.raw_overrides);
        let usage = run_main(inner, codex_linux_sandbox_exe).await?;
        if !usage.is_zero() {
            println!("{}", agcodex_core::protocol::FinalOutput::from(usage));
        }
        Ok(())
    })
}
