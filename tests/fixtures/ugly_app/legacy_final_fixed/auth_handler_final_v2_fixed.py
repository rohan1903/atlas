"""Sounds like the fix — still dead code."""

from legacy.auth_service import AuthService
from utils.logger import log_info


def login_handler(request):
    log_info("legacy_final_fixed login_handler")
    service = AuthService({})
    return service.login(request.body.get("email"), request.body.get("password"))
