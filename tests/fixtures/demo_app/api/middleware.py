"""HTTP middleware."""

from auth.token import verify_token
from utils.logger import log_error


def auth_middleware(request, next_handler):
    token = request.headers.get("Authorization")
    if not verify_token(token):
        log_error("unauthorized")
        return None
    return next_handler(request)
