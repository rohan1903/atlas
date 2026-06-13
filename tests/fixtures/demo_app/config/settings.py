"""Application configuration."""

DEFAULT_DATABASE_URL = "sqlite:///app.db"


def load_settings():
    return {
        "database_url": DEFAULT_DATABASE_URL,
        "jwt_secret": "demo-secret",
        "debug": False,
    }


def get_database_url(settings):
    return settings.get("database_url", DEFAULT_DATABASE_URL)
