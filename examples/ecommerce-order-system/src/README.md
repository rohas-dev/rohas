# src

Rohas event-driven application

## Project Structure

```
├── schema/          # Schema definitions (.ro files)
│   ├── api/        # API endpoint schemas
│   ├── events/     # Event schemas
│   ├── models/     # Data model schemas
│   └── cron/       # Cron job schemas
├── src/
│   ├── generated/  # Auto-generated types (DO NOT EDIT)
│   └── handlers/   # Your handler implementations
│       ├── api/    # API handlers
│       ├── events/ # Event handlers
│       └── cron/   # Cron job handlers
└── config/         # Configuration files
```

## Getting Started

### Installation

```bash
# Install dependencies (TypeScript)
npm install

# Or for Python
pip install -r requirements.txt
```

### Development

```bash
# Generate code from schema
rohas codegen

# Start development server
rohas dev

# Validate schema
rohas validate
```

## Schema Overview


### APIs

- `POST /orders` - CreateOrder
- `GET /orders/:orderId` - GetOrderStatus
- `POST /orders/:orderId/cancel` - CancelOrder
- `GET /orders` - ListOrders

### Events

- `OrderCreated` - Payload: Json
- `OrderCancelled` - Payload: Json
- `OrderStatusUpdated` - Payload: Json
- `OrderExpired` - Payload: Json
- `OrderShipped` - Payload: Json
- `OrderDelivered` - Payload: Json
- `PaymentProcessed` - Payload: Json
- `PaymentCompleted` - Payload: Json
- `PaymentFailed` - Payload: Json
- `PaymentRefunded` - Payload: Json
- `InventoryReserved` - Payload: Json
- `InventoryReleased` - Payload: Json
- `InventoryLowStock` - Payload: Json
- `InventoryOutOfStock` - Payload: Json

### Cron Jobs

- `SyncInventory` - Schedule: 0 */6 * * *
- `ExpirePendingOrders` - Schedule: */5 * * * *


## Handler Naming Convention

Handler files must be named exactly as the API/Event/Cron name in the schema:

- API `Health` → `src/handlers/api/Health.ts`
- Event `UserCreated` → Handler defined in event schema
- Cron `DailyCleanup` → `src/handlers/cron/DailyCleanup.ts`

## Generated Code

The `src/generated/` directory contains auto-generated TypeScript types and interfaces.
**DO NOT EDIT** these files manually - they will be regenerated when you run `rohas codegen`.

## Adding New Features

1. Define your schema in `schema/` directory
2. Run `rohas codegen` to generate types and handler stubs
3. Implement your handler logic in `src/handlers/`
4. Test with `rohas dev`

## Configuration

See `config/rohas.toml` for project configuration.

## License

MIT
