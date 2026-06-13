"""Third migration wave — still not wired."""

from utils.logger import log_info


def login_handler(request):
    log_info("legacy_final login_handler")
    return {"legacy_final": True}
