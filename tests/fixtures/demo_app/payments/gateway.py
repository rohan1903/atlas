"""External payment provider client."""


class PaymentGateway:
    def __init__(self, settings):
        self.api_key = settings.get("payment_api_key", "demo-key")

    def charge(self, email, amount):
        return {"status": "ok", "amount": amount, "email": email}

    def refund(self, payment_id):
        return {"status": "refunded", "payment_id": payment_id}
