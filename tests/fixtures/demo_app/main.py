"""Application entrypoint."""

from api.router import create_app
from config.settings import load_settings
from utils.logger import setup_logging


def main():
    setup_logging()
    settings = load_settings()
    app = create_app(settings)
    return app


if __name__ == "__main__":
    main()
