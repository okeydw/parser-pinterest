use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use thirtyfour::prelude::*;
use tokio::sync::Semaphore;
use tokio::time::Duration;
use clap::{Parser, Subcommand};

// ====================== CLI ======================

#[derive(Parser, Debug)]
#[command(
    name = "parser",
    version = "0.7.0",
    about = "Pinterest Parser",
    long_about = "\
Pinterest Parser
──────────────────────────────
Инструмент для скачивания изображений с Pinterest.

Доступные режимы:
  auth     - авторизованный режим, без ограничений на количество
  limited  - без авторизации, максимум 30 картинок
  relay    - без авторизации, обходит лимит переходя по постам

Примеры:
  parser.exe auth -a 200 --email you@gmail.com --password pass
  parser.exe limited -a 30 -d ./output
  parser.exe relay -a 100 --cooldown 1000",
    after_help = "Подробнее по каждому режиму: parser.exe <mode> --help",
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true,
)]
struct Cli {
    #[arg(
        short = 'h', long = "help",
        help = "Показать справку по программе",
        action = clap::ArgAction::Help,
        global = true,
    )]
    help: (),

    #[arg(
        short = 'V', long = "version",
        help = "Показать версию программы",
        action = clap::ArgAction::Version,
    )]
    version: (),

    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    #[command(
        about = "Авторизованный режим - *почти без ограничений",
        long_about = "\
Режим: auth
──────────────────────────────
Выполняет вход в аккаунт Pinterest перед началом сбора.
Снимает ограничение в ~30 пинов для неавторизованных пользователей.

Примеры:
  parser.exe auth --email you@gmail.com --password pass
  parser.exe auth -a 500 -e you@gmail.com --password pass -d ./photos",
        after_help = "Режим имеет ограничение только по кол-ву пинов на странице! ",
        disable_help_flag = true,
    )]
    Auth {
        #[arg(
            short = 'h', long = "help",
            help = "Показать справку по режиму auth",
            action = clap::ArgAction::Help,
        )]
        help: (),
        #[command(flatten)]
        common: CommonArgs,
        #[arg(short, long, help = "Email аккаунта Pinterest")]
        email: String,
        #[arg(long, help = "Пароль аккаунта Pinterest")]
        password: String,
    },

    #[command(
        about = "Без авторизации - максимум 30 пинов",
        long_about = "\
Режим: limited
──────────────────────────────
Работает без входа в аккаунт.
Pinterest ограничивает просмотр пинов для неавторизованных юзеров.
Примерно 30 пинов за раз.

Примеры:
  parser.exe limited
  parser.exe limited -a 20 -d ./output --cooldown 500",
        after_help = "Если вы используете этот режим и
обработка пинов зависла, то завершайте процесc и используйте relay!",
        disable_help_flag = true,
    )]
    Limited {
        #[arg(
            short = 'h', long = "help",
            help = "Показать справку по режиму limited",
            action = clap::ArgAction::Help,
        )]
        help: (),
        #[command(flatten)]
        common: CommonArgs,
    },

    #[command(
        about = "Без авторизации - обходит лимит через переходы по постам",
        long_about = "\
Режим: relay
──────────────────────────────
Работает без входа в аккаунт.
Собирает до 30 пинов со стартовой страницы, затем переходит
по 2-му найденному посту и продолжает сбор с новой страницы.
Повторяет до достижения нужного количества.

Примеры:
  parser.exe relay -a 100
  parser.exe relay -a 200 -u https://ru.pinterest.com/search/pins/?q=cats -d ./cats",
        after_help = "Имеет шанс сбоя по контексту!\nДубликаты между страницами не отслеживаются!",
        disable_help_flag = true,
    )]
    Relay {
        #[arg(
            short = 'h', long = "help",
            help = "Показать справку по режиму relay",
            action = clap::ArgAction::Help,
        )]
        help: (),
        #[command(flatten)]
        common: CommonArgs,
    },
}

#[derive(Parser, Debug, Clone)]
#[command(disable_help_flag = true)]
struct CommonArgs {
    #[arg(
        short, long,
        default_value = "https://ru.pinterest.com/ideas/",
        hide_default_value = true,
        help = "Стартовый URL Pinterest'a",
        long_help = "Стартовый URL Pinterest'a. Может быть любой страницей, где есть пины [https://ru.pinterest.com/]"
    )]
    url: String,

    #[arg(
        short, long,
        default_value_t = 10,
        hide_default_value = true,
        help = "Количество картинок для скачивания",
        long_help = "Количество картинок. В режиме limited автоматически ограничивается до 30. [10]"
    )]
    amount: usize,

    #[arg(
        short, long,
        default_value_t = 800,
        hide_default_value = true,
        help = "Пауза между скроллами страницы в мс",
        long_help = "Пауза между скроллами страницы в мс. Меньше = быстрее, но выше риск блокировки. [800]"
    )]
    cooldown: u64,


    #[cfg(target_os = "windows")]
    #[arg(
        short, long,
        default_value = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        hide_default_value = true,
        help = "Путь к Chrome браузеру",
        long_help = "Путь к браузеру Chrome, чья версия такая же как и chromedriver. [C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe]")]
    binary: String,

    #[cfg(not(target_os = "windows"))]
    #[arg(
        short, long,
        default_value = "/usr/bin/google-chrome",
        help = "Путь к исполняемому файлу Google Chrome"
    )]
    binary: String,

    #[arg(
        short, long,
        default_value_t = 9515,
        hide_default_value = true,
        help = "Порт сервиса ChromeDriver",
        long_help = "Порт ChromeDriver (Порт ChromeDriver (можете задать порт у chromedriver с аргументов --port 9515)) [9515]"
    )]
    port: u16,

    #[arg(
        short, long,
        default_value = "pinterest_data",
        hide_default_value = true,
        help = "Папка для сохранения картинок",
        long_help = "Папка для сохранения картинок (создаётся автоматически) [./pinterest_data]"
    )]
    dest: PathBuf,

    #[arg(
        short = 'j', long,
        default_value_t = 15,
        hide_default_value = true,
        help = "Количество параллельных потоков",
        long_help = "Количество параллельных потоков загрузки (рекомендуется 10–20) [15]"
    )]
    jobs: usize,

    #[arg(
        short = 'A', long,
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        hide_default_value = true,
        help = "User-Agent для HTTP запросов",
        long_help = "User-Agent для HTTP запросов [Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36]"
    )]
    user_agent: String,
}
// ====================== MAIN ======================

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    match &cli.mode {
        Mode::Auth { common, email, password, .. } => {
            print_header(common, "auth");
            let driver = make_driver(&common.binary, common.port).await?;
            login_pinterest(&driver, email, password).await?;
            driver.goto(&common.url).await?;
            tokio::time::sleep(Duration::from_millis(2000)).await;
            let result = collect_pins(&driver, common.amount, common.cooldown).await?;
            driver.quit().await.ok();
            let (_, images) = result;
            finish(images, common).await;
        }

        Mode::Limited { common, .. } => {
            let amount = common.amount.min(30);
            if common.amount > 30 {
                println!("[!] Режим limited: максимум 30 картинок, amount=30");
            }
            print_header(common, "limited");
            let driver = make_driver(&common.binary, common.port).await?;
            driver.goto(&common.url).await?;
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let result = collect_pins(&driver, amount, common.cooldown).await?;
            driver.quit().await.ok();
            let (_, images) = result;
            finish(images, common).await;
        }

        Mode::Relay { common, .. } => {
            print_header(common, "relay");
            let driver = make_driver(&common.binary, common.port).await?;
            let images = collect_relay(&driver, &common.url, common.amount, common.cooldown).await;
            driver.quit().await.ok();

            match images {
                Ok(images) if images.is_empty() => {
                    eprintln!("[!] Ничего не найдено. Проверьте URL и убедитесь что страница содержит пины.");
                }
                Ok(images) => {
                    println!("\n[V] Собрано {} картинок, начата загрузка...\n", images.len());
                    download_images(&images, &common.dest, common.jobs, &common.user_agent).await;
                }
                Err(e) => {
                    eprintln!("[X] Ошибка парсинга: {}", e);
                }
            }
        }
    }

    Ok(())
}

// ====================== DRIVER ======================

async fn make_driver(binary: &str, port: u16) -> Result<WebDriver, Box<dyn Error + Send + Sync>> {
    let server_url = format!("http://localhost:{}", port);
    let mut caps = DesiredCapabilities::chrome();
    caps.set_binary(binary).expect("Не удалось задать путь к Chrome");
    caps.add_arg("--headless=new").expect("...");
    caps.add_arg("--no-sandbox").expect("...");
    caps.add_arg("--disable-dev-shm-usage").expect("...");
    caps.add_arg("--disable-gpu").expect("...");
    caps.add_arg("--disable-extensions").expect("...");
    caps.add_arg("--log-level=0").expect("...");
    caps.add_arg("--disable-background-networking").expect("...");

    let driver = WebDriver::new(&server_url, caps).await?;
    driver.set_implicit_wait_timeout(Duration::from_secs(2)).await?;
    Ok(driver)
}

// ====================== LOGIN ======================

async fn login_pinterest(
    driver: &WebDriver,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Авторизация...");
    driver.goto("https://ru.pinterest.com/login").await?;
    tokio::time::sleep(Duration::from_millis(2000)).await;

    driver.find(By::Id("email")).await?.send_keys(email).await?;
    driver.find(By::Id("password")).await?.send_keys(password).await?;
    driver.find(By::Css(r#"button[type="submit"]"#)).await?.click().await?;

    tokio::time::sleep(Duration::from_millis(3000)).await;
    println!("Авторизация успешна");
    Ok(())
}

// ====================== COLLECT ======================

async fn collect_pins(
    driver: &WebDriver,
    amount: usize,
    cooldown: u64,
) -> Result<(Vec<String>, Vec<String>), Box<dyn Error + Send + Sync>> {
    let mut posts:  Vec<String> = Vec::new();
    let mut images: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stuck_count = 0u32;

    while images.len() < amount {
        let prev_count = images.len();

        for pin in driver.find_all(By::Css(r#"[role="listitem"]"#)).await? {
            if images.len() >= amount { break; }
            if let Some((href, src)) = extract_pin(&pin).await {
                if seen.contains(&href) { continue; }
                seen.insert(href.clone());
                println!("Найдено [{}/{}]: {}", images.len() + 1, amount, href);
                posts.push(format!("https://ru.pinterest.com{}", href));
                images.push(src.replace("236x", "736x"));
            }
        }

        if images.len() < amount {
            if images.len() == prev_count {
                stuck_count += 1;
                if stuck_count >= 3 {
                    driver.execute("window.scrollTo(0, 0);", Vec::new()).await?;
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    driver.execute("window.scrollTo(0, document.body.scrollHeight);", Vec::new()).await?;
                    tokio::time::sleep(Duration::from_millis(cooldown + 1500)).await;
                    stuck_count = 0;
                } else {
                    tokio::time::sleep(Duration::from_millis(cooldown + 1500)).await;
                }
            } else {
                stuck_count = 0;
                tokio::time::sleep(Duration::from_millis(cooldown)).await;
            }
            driver.execute("window.scrollBy(0, window.innerHeight * 2);", Vec::new()).await?;
        }
    }

    Ok((posts, images))
}

async fn collect_relay(
    driver: &WebDriver,
    start_url: &str,
    amount: usize,
    cooldown: u64,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut images: Vec<String> = Vec::new();
    let mut current_url = start_url.to_string();

    while images.len() < amount {
        println!("\n→ Страница: {}", current_url);
        driver.goto(&current_url).await?;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let batch_limit = (amount - images.len()).min(30);
        let mut batch_posts: Vec<String> = Vec::new();
        let mut batch_images: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut stuck_count = 0u32;

        while batch_images.len() < batch_limit {
            let prev_count = batch_images.len();

            for pin in driver.find_all(By::Css(r#"[role="listitem"]"#)).await? {
                if batch_images.len() >= batch_limit { break; }
                if let Some((href, src)) = extract_pin(&pin).await {
                    if seen.contains(&href) { continue; }
                    seen.insert(href.clone());
                    println!("Найдено [{}/{}]: {}", images.len() + batch_images.len() + 1, amount, href);
                    batch_posts.push(format!("https://ru.pinterest.com{}", href));
                    batch_images.push(src.replace("236x", "736x"));
                }
            }

            if batch_images.len() < batch_limit {
                if batch_images.len() == prev_count {
                    stuck_count += 1;
                    if stuck_count >= 3 { break; } // выходим — страница кончилась
                    tokio::time::sleep(Duration::from_millis(cooldown + 1500)).await;
                } else {
                    stuck_count = 0;
                    tokio::time::sleep(Duration::from_millis(cooldown)).await;
                }
                driver.execute("window.scrollBy(0, window.innerHeight * 2);", Vec::new()).await?;
            }
        }

        images.extend(batch_images);

        // Берём 2-й пост как следующую страницу (если он есть)
        if images.len() < amount {
            if batch_posts.len() >= 2 {
                current_url = batch_posts[1].clone();
            } else if !batch_posts.is_empty() {
                current_url = batch_posts[0].clone();
            } else {
                println!("Нет постов для перехода, завершаю.");
                break;
            }
        }
    }

    Ok(images)
}


async fn extract_pin(pin: &WebElement) -> Option<(String, String)> {
    let href = pin.find(By::Css(r#"a[href*="/pin/"]"#)).await.ok()?
        .attr("href").await.ok()?? ;
    let src  = pin.find(By::Tag("img")).await.ok()?
        .attr("src").await.ok()??;
    Some((href, src))
}

fn print_header(common: &CommonArgs, mode: &str) {
    println!("Запуск парсера...");
    println!("  Режим:    {}", mode);
    println!("  URL:      {}", common.url);
    println!("  Кол-во:   {}", common.amount);
    println!("  Кулдаун:  {} мс", common.cooldown);
    println!("  Chrome:   {}", common.binary);
    println!("  Порт:     {}", common.port);
    println!("  Папка:    {}", common.dest.display());
    println!("  Потоки:   {}", common.jobs);
    println!();
}

async fn finish(images: Vec<String>, common: &CommonArgs) {
    println!("\nСобрано {} постов, начата загрузка...\n", images.len());
    download_images(&images, &common.dest, common.jobs, &common.user_agent).await;
}


async fn download_images(images: &[String], dest: &PathBuf, jobs: usize, user_agent: &str) {
    if let Err(e) = tokio::fs::create_dir_all(dest).await {
        eprintln!("[X] Не удалось создать папку {}: {}", dest.display(), e);
        return;
    }

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Referer", "https://ru.pinterest.com/".parse().unwrap());
    headers.insert("Accept", "image/webp,image/apng,image/*,*/*;q=0.8".parse().unwrap());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent)
        .default_headers(headers)
        .build()
        .expect("Не удалось создать HTTP клиент");

    let semaphore = Arc::new(Semaphore::new(jobs));
    let mut handles = Vec::new();

    for (i, url) in images.iter().enumerate() {
        let url    = url.clone();
        let dest   = dest.clone();
        let client = client.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let handle = tokio::spawn(async move {
            let _permit = permit;

            let filename = url
                .split('/').last()
                .and_then(|s| s.split('?').next())
                .filter(|s| !s.is_empty())
                .map(|s| format!("{:03}_{}", i + 1, s))
                .unwrap_or_else(|| format!("{:03}.jpg", i + 1));

            let filepath = dest.join(&filename);

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.bytes().await {
                        Ok(bytes) => match tokio::fs::write(&filepath, &bytes).await {
                            Ok(_)  => println!("[{:03}] v {}", i + 1, filename),
                            Err(e) => eprintln!("[{:03}] x Ошибка записи {}: {}", i + 1, filename, e),
                        },
                        Err(e) => eprintln!("[{:03}] x Ошибка чтения байт: {}", i + 1, e),
                    }
                }
                Ok(resp) => eprintln!("[{:03}] x HTTP {} для {}", i + 1, resp.status(), url),
                Err(e)   => eprintln!("[{:03}] x Ошибка запроса: {}", i + 1, e),
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    println!("\nЗагрузка завершена.");
}