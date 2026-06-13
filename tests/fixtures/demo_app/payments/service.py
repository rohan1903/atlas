"""Payment processing."""

from payments.gateway import PaymentGateway
from users.repository import UserRepository
from utils.logger import log_info


class PaymentService:
    def __init__(self, settings):
        self.gateway = PaymentGateway(settings)
        self.users = UserRepository(settings)

    def charge_user(self, user_id, amount):
        log_info("PaymentService.charge_user")
        user = self.users.get_by_id(user_id)
        return self.gateway.charge(user["email"], amount)

    def refund_user(self, user_id, payment_id):
        return self.gateway.refund(payment_id)
