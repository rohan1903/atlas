"""Original auth stack — superseded, never wired from main."""

from legacy_final_fixed.auth_service_final_v2_fixed import AuthServiceFinalV2Fixed
from utils.logger import log_info


def login_handler(request):
    """Dead handler — looks important but is not registered."""
    log_info("legacy login_handler")
    service = AuthServiceFinalV2Fixed({})
    return service.login(request.body.get("email"), request.body.get("password"))
