"""Main HTTP router — only canonical auth is wired."""

from auth.routes import login_handler, register_auth_routes
from auth.service import AuthService
from users.service import UserService
from utils.logger import log_info


def create_app(settings):
    router = Router(settings)
    if settings.get("USE_NEW_AUTH", True):
        register_auth_routes(router)
        router.add_route("POST", "/login", login_handler)
    # Legacy auth stacks are intentionally not registered.
    return router


class Router:
    def __init__(self, settings):
        self.settings = settings
        self.auth_service = AuthService(settings)
        self.user_service = UserService(settings)

    def add_route(self, method, path, handler):
        log_info(f"registered {method} {path}")
        return handler


def list_users_handler(request):
    service = UserService({})
    return service.list_users()
