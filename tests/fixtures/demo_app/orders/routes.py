"""Order HTTP routes."""

from orders.service import OrderService
from utils.logger import log_info


class route:
    @staticmethod
    def post(path):
        def wrap(handler):
            return handler
        return wrap


@route.post("/orders")
def create_order_handler(request):
    service = OrderService(request.settings)
    user_id = request.user_id
    items = request.body.get("items", [])
    log_info("create order request")
    return service.place_order(user_id, items)


def list_orders_handler(request):
    service = OrderService(request.settings)
    return service.list_orders(request.user_id)
