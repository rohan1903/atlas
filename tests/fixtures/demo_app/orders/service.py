"""Order business logic."""

from orders.repository import OrderRepository
from payments.service import PaymentService
from users.repository import UserRepository
from utils.logger import log_info, log_error


class OrderService:
    def __init__(self, settings):
        self.orders = OrderRepository(settings)
        self.users = UserRepository(settings)
        self.payments = PaymentService(settings)

    def place_order(self, user_id, items):
        log_info("OrderService.place_order")
        user = self.users.get_by_id(user_id)
        if not user:
            log_error("unknown user")
            return None
        order = self.orders.create_order(user_id, items)
        charge = self.payments.charge_user(user_id, order["id"], total_for(items))
        if not charge.get("ok"):
            self.orders.update_status(order["id"], "failed")
            return None
        self.orders.update_status(order["id"], "paid")
        return order

    def get_order(self, order_id):
        return self.orders.get_by_id(order_id)

    def list_orders(self, user_id):
        return self.orders.list_for_user(user_id)


def total_for(items):
    return sum(item.get("price", 0) for item in items)
