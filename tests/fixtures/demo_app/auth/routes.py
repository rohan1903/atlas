"""Authentication HTTP routes."""

from auth.service import AuthService
from utils.logger import log_info


class route:
    @staticmethod
    def post(path):
        def wrap(handler):
            return handler
        return wrap


def register_auth_routes(router):
    router.add_route("POST", "/register", register_handler)
    router.add_route("POST", "/logout", logout_handler)


@route.post("/login")
def login_handler(request):
    """Handle POST /login."""
    service = AuthService(request.settings)
    email = request.body.get("email")
    password = request.body.get("password")
    log_info("login attempt")
    return service.login(email, password)


def register_handler(request):
    service = AuthService(request.settings)
    return service.register(request.body)


def logout_handler(request):
    service = AuthService(request.settings)
    return service.logout(request.user_id)
