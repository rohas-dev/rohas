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

- `GET /health` - Health
- `POST /users` - CreateUser
- `GET /test` - Test
- `GET /timeline/fast` - TimelineTestFast
- `GET /timeline/slow` - TimelineTestSlow
- `GET /timeline/very-slow` - TimelineTestVerySlow
- `GET /timeline/multi-step` - TimelineTestMultiStep

### Events

- `FastCompleted` - Payload: Json
- `SlowCompleted` - Payload: Json
- `VerySlowCompleted` - Payload: Json
- `BottleneckDetected` - Payload: Json
- `MajorBottleneckDetected` - Payload: Json
- `ValidationComplete` - Payload: Json
- `ProcessingComplete` - Payload: Json
- `ExternalCallComplete` - Payload: Json
- `FinalizationComplete` - Payload: Json
- `CleanupStep1` - Payload: Json
- `CleanupStep2` - Payload: Json
- `BottleneckLogged` - Payload: Json
- `WelcomeEmailSent` - Payload: Json
- `UserCreated` - Payload: User
- `ManualTrigger` - Payload: String

### Cron Jobs

- `DailyCleanup` - Schedule: 0 */5 * * * *


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
