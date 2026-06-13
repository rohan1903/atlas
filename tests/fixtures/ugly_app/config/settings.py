"""Runtime configuration and feature flags."""

DEFAULT_SETTINGS = {
    "USE_NEW_AUTH": True,
    "USE_LEGACY_AUTH": False,
    "ENABLE_TEMP_ROUTES": False,
}


def load_settings():
    return dict(DEFAULT_SETTINGS)
