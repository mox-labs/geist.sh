"""CLI entry point for geist-run."""

from __future__ import annotations

from pathlib import Path

import click


@click.group()
@click.version_option(package_name="geist-run")
def main() -> None:
    """geist-run — agent runtime for geist.sh."""


@main.command()
@click.argument("project_path", default=".", type=click.Path(exists=True, path_type=Path))
def bodhi(project_path: Path) -> None:
    """Start a bodhi interview for the project at PROJECT_PATH."""
    from geist_run.agents.bodhi import create_bodhi_agent

    agent = create_bodhi_agent(project_path)
    agent.run()


if __name__ == "__main__":
    main()
