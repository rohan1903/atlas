"""Abandoned route table — ENABLE_TEMP_ROUTES is False in settings."""

from legacy.auth_handler import login_handler as legacy_login
from utils.logger import log_info


def register_old_auth_routes(router):
    router.add_route("POST", "/login-old", legacy_login)
    router.add_route("POST", "/login-legacy", legacy_login)
    log_info("registered temp auth routes")
