import structlog
import typer

SECTOR_SIZE = 4096

log = structlog.get_logger()


def main(log_level: str = "info"):
    structlog.configure(
        wrapper_class=structlog.make_filtering_bound_logger(log_level),
    )
    log.info("Hello from superflat!")


def cli():
    typer.run(main)


if __name__ == "__main__":
    main()
