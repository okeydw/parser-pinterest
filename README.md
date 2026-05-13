# Pinterest Parser

![Pinterest Parser](https://i.pinimg.com/736x/35/60/3b/35603b2bb13304e8a3c380789ca1e2f6.jpg)

Инструмент для скачивания изображений с Pinterest через ChromeDriver.

---

## Требования

- [Google Chrome](https://www.google.com/chrome/)
- [ChromeDriver](https://googlechromelabs.github.io/chrome-for-testing/) - версия должна совпадать с Chrome

Перед запуском парсера запустите ChromeDriver:
```bash
chromedriver.exe --port=9515
```

---

## Установка

```bash
git clone https://github.com/okeydw/parser-pinterest
cd pinterest-parser
cargo build --release
```

---

## Режимы

### `auth` - авторизованный режим
Выполняет вход в аккаунт Pinterest. Снимает ограничение ~30 пинов.
```bash
parser.exe auth --email you@gmail.com --password pass
parser.exe auth -a 200 -e you@gmail.com --password pass -d ./photos
```

### `limited` - без авторизации
Работает без входа в аккаунт. Максимум ~30 пинов за сессию.
```bash
parser.exe limited
parser.exe limited -a 20 -d ./output -u "https://ru.pinterest.com/ideas/"
```

### `relay` - без авторизации, обход лимита
Собирает до 30 пинов, затем переходит по найденному посту и продолжает.
Дубликаты между страницами не отслеживаются.
```bash
parser.exe relay -a 100
parser.exe relay -a 200 -u "https://ru.pinterest.com/ideas/" -d ./output
```

---

## Аргументы

| Флаг | Описание | По умолчанию |
|------|----------|--------------|
| `-u`, `--url` | Стартовая страница Pinterest | `https://ru.pinterest.com/ideas/` |
| `-a`, `--amount` | Количество картинок | `10` |
| `-c`, `--cooldown` | Пауза между скроллами (мс) | `800` |
| `-b`, `--binary` | Путь к Chrome | `C:\Program Files\...\chrome.exe` |
| `-p`, `--port` | Порт ChromeDriver | `9515` |
| `-d`, `--dest` | Папка для сохранения | `pinterest_data` |
| `-j`, `--jobs` | Параллельных загрузок | `15` |
| `-A`, `--user-agent` | User-Agent | `Mozilla/5.0 ...` |

---

## Справка

```bash
parser.exe --help
parser.exe auth --help
parser.exe limited --help
parser.exe relay --help
```
