"""Main HTTP router."""

from auth.routes import register_auth_routes
from auth.service import AuthService
from orders.routes import create_order_handler, list_orders_handler
from users.service import UserService
from utils.logger import log_info


def create_app(settings):
    router = Router(settings)
    register_auth_routes(router)
    router.add_route("GET", "/users", list_users_handler)
    router.add_route("POST", "/orders", create_order_handler)
    router.add_route("GET", "/orders", list_orders_handler)
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
