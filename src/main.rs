// =======================
// AXUM CORE
// =======================
use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put, delete},
    Json, Router,
};

// =======================
// TOWER (CORS + STATIC FILES)
// =======================
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

// =======================
// SERIALIZATION
// =======================
use serde::{Deserialize, Serialize};
use serde_json::json;

// =======================
// DATABASE (SQLX)
// =======================
use sqlx::{FromRow, SqlitePool};
use sqlx::sqlite::SqlitePoolOptions;

// =======================
// UTILS / GENERAL
// =======================
use uuid::Uuid;
use chrono::Utc;
use dotenv::dotenv;
use reqwest::Client;

use std::{
    collections::HashMap,
    env,
    fs,
    net::SocketAddr,
    sync::Arc,
    path::Path as StdPath,
};

// =======================
// Estructuras de datos
// =======================

#[derive(Debug, Serialize, Deserialize)]
struct BannerInput {
    nombre: String,
    referencia: Option<String>,
    costo: f64,
    archivo_imagen: String,
    video_url: Option<String>,
    button_text: Option<String>,
    activo: i32,
    orden: i32,
}

#[derive(Debug, Serialize)]
struct UploadBannerImageResponse {
    success: bool,
    archivo_imagen: String,
    image_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
struct Producto {
    referencia: String,
    categoria: String,
    precio: f64,
    fecha_venta: String,
    imagen: String,
    cantidad: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
struct Banner {
    id: i32,
    nombre: String,
    referencia: Option<String>,
    costo: f64,
    archivo_imagen: String,
    video_url: Option<String>,
    button_text: Option<String>,
    clicks: i32,
    activo: i32,
    orden: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
struct Lead {
    id: i64,
    nombre: String,
    telefono: String,
    ciudad: Option<String>,
    canal: String,
    producto_referencia: Option<String>,
    mensaje: Option<String>,
    estado: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
struct ProductoImagen {
    id: i64,
    producto_referencia: String,
    imagen_url: String,
    orden: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct NuevoProducto {
    referencia: String,
    categoria: String,
    precio: f64,
    imagen: String,
    cantidad: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct NuevoLead {
    nombre: String,
    telefono: String,
    ciudad: Option<String>,
    canal: String,
    producto_referencia: Option<String>,
    mensaje: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NuevaProductoImagen {
    producto_referencia: String,
    imagen_url: String,
    orden: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct MensajeUsuario {
    mensaje: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckoutItem {
    referencia: String,
    cantidad: i32,
    precio: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckoutRequest {
    items: Vec<CheckoutItem>,
    total: f64,
    currency: String,
    customer_email: Option<String>,
    customer_name: Option<String>,
    customer_phone: Option<String>,
    payment_method: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckoutResponse {
    success: bool,
    checkout_url: String,
    reference: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StockBajoItem {
    referencia: String,
    categoria: String,
    precio: f64,
    imagen: String,
    cantidad: i32,
}

impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for StockBajoItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            referencia: row.try_get("referencia")?,
            categoria: row.try_get("categoria")?,
            precio: row.try_get("precio")?,
            imagen: row.try_get("imagen")?,
            cantidad: row.try_get("cantidad")?,
        })
    }
}

#[derive(Debug, Serialize)]
struct MetricasResponse {
    total_productos: i64,
    valor_inventario: f64,
    total_leads: i64,
    leads_hoy: i64,
    banners_clicks_total: i64,
    top_productos_stock: Vec<StockBajoItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct YoutubeViewsQuery {
    videoId: String,
}

// =======================
// Estado compartido - LIMPIO SIN BINANCE
// =======================
#[derive(Clone)]
struct AppState {
    db: Arc<SqlitePool>,
    base_url: String,
    admin_token: String,
    wompi_public_key: String,
    wompi_private_key: String,
    wompi_integrity_key: String,
    youtube_api_key: String,
}

// =======================
// Main
// =======================
#[tokio::main]
async fn main() {
    dotenv::from_path("/home/javier/api-rust/.env").ok();
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL no está definido");
    let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_else(|_| "cambia-esto-por-un-token-seguro".to_string());
    let wompi_public_key = env::var("WOMPI_PUBLIC_KEY").unwrap_or_else(|_| "".to_string());
    let wompi_private_key = env::var("WOMPI_PRIVATE_KEY").unwrap_or_else(|_| "".to_string());
    let wompi_integrity_key = env::var("WOMPI_INTEGRITY_KEY").unwrap_or_else(|_| "".to_string());
    let youtube_api_key = env::var("YOUTUBE_API_KEY").unwrap_or_else(|_| "".to_string());

    println!("🟢 DB URL => {}", database_url);
    println!("🟢 BASE_URL => {}", base_url);

    fs::create_dir_all("./static/images").expect("No se pudo crear ./static/images");
    fs::create_dir_all("./static/products").expect("No se pudo crear ./static/products");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("No se pudo conectar a la DB");

    let state = AppState {
        db: Arc::new(pool),
        base_url,
        admin_token,
        wompi_public_key,
        wompi_private_key,
        wompi_integrity_key,
        youtube_api_key,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .route("/", get(root))
        .route("/saludo", get(saludo))
        .route("/producto", post(crear_producto))
        .route("/productos", get(obtener_productos))
        .route("/buscar", get(buscar_producto))
        .route("/recomendados", get(recomendar_productos))
        .route("/banners", get(obtener_banners))
        .route("/chatbot", post(chatbot))
        .route("/descargar-db", get(descargar_db))
        .route("/upload_banner", post(subir_banners))
        .route("/upload_producto_imagen", post(subir_imagen_producto))
        .route("/banners/click/{id}", post(click_banner))
        .route("/lead", post(crear_lead))
        .route("/metricas", get(obtener_metricas))
        .route("/stock_bajo", get(obtener_stock_bajo))
        .route("/producto_imagenes", get(obtener_producto_imagenes))
        .route("/producto_imagenes", post(crear_producto_imagen))
        .route("/producto_imagen/{id}", post(eliminar_producto_imagen))
        .route("/admin/upload-banner-image", post(subir_banner_imagen_admin))
        .route("/admin/banners", post(crear_banner_admin))
        .route("/admin/banners/{id}", put(actualizar_banner_admin))
        .route("/admin/banners/{id}", delete(eliminar_banner_admin))
        .route("/checkout/create", post(crear_checkout))
        .route("/youtube/views", get(obtener_vistas_youtube)) // NUEVO - Vistas reales YouTube
        .route("/api/health", get(api_health))
        .nest_service("/static", ServeDir::new("./static"))
        .with_state(state.clone())
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Servidor corriendo en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("No se pudo enlazar el puerto");
    axum::serve(listener, app).await.expect("Error al iniciar el servidor");
}

// =======================
// Helpers
// =======================
fn validate_admin(headers: &HeaderMap, expected_token: &str) -> Result<(), Response> {
    let auth_header = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", expected_token);
    if auth_header != expected {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({ "error": "No autorizado" }))).into_response());
    }
    Ok(())
}

fn sanitize_filename(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() { out.push(ch); }
        else if ch == ' ' || ch == '-' || ch == '_' { out.push('_'); }
    }
    while out.contains("__") { out = out.replace("__", "_"); }
    out.trim_matches('_').to_string()
}

fn normalize_extension(ext: &str) -> &str {
    match ext.to_lowercase().as_str() {
        "jpeg" => "jpg", "jpg" => "jpg", "png" => "png", "webp" => "webp", _ => "jpg",
    }
}

async fn root() -> &'static str { "¡Hola desde Rust y Axum! - MontiTech API" }
async fn saludo() -> Json<serde_json::Value> { Json(json!({ "mensaje": "Hola, bienvenido a mi API" })) }
async fn api_health() -> Json<serde_json::Value> { Json(json!({ "status": "ok", "service": "montitech-api" })) }

// =======================
// NUEVO: Vistas reales de YouTube
// =======================
async fn obtener_vistas_youtube(
    State(state): State<AppState>,
    Query(params): Query<YoutubeViewsQuery>,
) -> impl IntoResponse {
    if state.youtube_api_key.is_empty() {
        return (StatusCode::OK, Json(json!({ "viewCount": 0, "error": "YOUTUBE_API_KEY no configurada" }))).into_response();
    }

    // Si el videoId viene como URL completa, extrae solo el ID
    let video_id = if params.videoId.contains("v=") {
        params.videoId.split("v=").last().unwrap_or(&params.videoId).split('&').next().unwrap_or(&params.videoId).to_string()
    } else if params.videoId.contains("youtu.be/") {
        params.videoId.split("youtu.be/").last().unwrap_or(&params.videoId).split('?').next().unwrap_or(&params.videoId).to_string()
    } else {
        params.videoId.clone()
    };

    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=statistics&id={}&key={}",
        video_id, state.youtube_api_key
    );

    let client = Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                let view_count = json_data["items"][0]["statistics"]["viewCount"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                return (StatusCode::OK, Json(json!({ "viewCount": view_count, "videoId": video_id }))).into_response();
            }
            (StatusCode::OK, Json(json!({ "viewCount": 0 }))).into_response()
        }
        Err(_) => (StatusCode::OK, Json(json!({ "viewCount": 0 }))).into_response(),
    }
}

// =======================
// Banners Admin
// =======================
async fn subir_banner_imagen_admin(State(state): State<AppState>, headers: HeaderMap, mut multipart: Multipart) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) { return resp; }
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No se recibió archivo" }))).into_response(),
    };
    let original_name = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| "banner.jpg".to_string());
    let data = match field.bytes().await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": format!("Error leyendo: {}", e) }))).into_response(),
    };
    let stem = StdPath::new(&original_name).file_stem().and_then(|s| s.to_str()).unwrap_or("banner");
    let ext = normalize_extension(StdPath::new(&original_name).extension().and_then(|s| s.to_str()).unwrap_or("jpg"));
    let unique_name = format!("{}_{}.{}", sanitize_filename(stem), Uuid::new_v4(), ext);
    let ruta = format!("./static/images/{}", unique_name);
    if let Err(e) = fs::write(&ruta, &data) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("No se pudo guardar: {}", e) }))).into_response();
    }
    let image_url = format!("{}/static/images/{}", state.base_url.trim_end_matches('/'), unique_name);
    (StatusCode::OK, Json(UploadBannerImageResponse { success: true, archivo_imagen: unique_name, image_url })).into_response()
}

async fn eliminar_banner_admin(Path(id): Path<i32>, State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) { return resp; }
    let _ = sqlx::query("DELETE FROM banners WHERE id = ?").bind(id).execute(&*state.db).await;
    (StatusCode::OK, Json(json!({ "success": true, "mensaje": "Banner eliminado" }))).into_response()
}

async fn crear_banner_admin(State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<BannerInput>) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) { return resp; }
    let result = sqlx::query("INSERT INTO banners (nombre, referencia, costo, archivo_imagen, video_url, button_text, clicks, activo, orden) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)")
        .bind(&payload.nombre).bind(&payload.referencia).bind(payload.costo).bind(&payload.archivo_imagen)
        .bind(&payload.video_url).bind(payload.button_text.as_deref().unwrap_or("Ver demostración")).bind(payload.activo).bind(payload.orden)
        .execute(&*state.db).await;
    match result {
        Ok(r) => (StatusCode::OK, Json(json!({ "success": true, "id": r.last_insert_rowid() }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("{}", e) }))).into_response(),
    }
}

async fn actualizar_banner_admin(Path(id): Path<i32>, State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<BannerInput>) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) { return resp; }
    let _ = sqlx::query("UPDATE banners SET nombre = ?, referencia = ?, costo = ?, archivo_imagen = ?, video_url = ?, button_text = ?, activo = ?, orden = ? WHERE id = ?")
        .bind(&payload.nombre).bind(&payload.referencia).bind(payload.costo).bind(&payload.archivo_imagen)
        .bind(&payload.video_url).bind(payload.button_text.as_deref().unwrap_or("Ver demostración")).bind(payload.activo).bind(payload.orden).bind(id)
        .execute(&*state.db).await;
    (StatusCode::OK, Json(json!({ "success": true }))).into_response()
}

// =======================
// Productos
// =======================
async fn crear_producto(State(state): State<AppState>, Json(producto): Json<NuevoProducto>) -> impl IntoResponse {
    let fecha_actual = Utc::now().to_rfc3339();
    let resultado = sqlx::query("INSERT OR REPLACE INTO productos (referencia, categoria, precio, fecha_venta, imagen, cantidad) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&producto.referencia).bind(&producto.categoria).bind(producto.precio).bind(&fecha_actual).bind(&producto.imagen).bind(producto.cantidad)
        .execute(&*state.db).await;
    match resultado {
        Ok(_) => (StatusCode::OK, Json(json!({ "mensaje": "Producto guardado" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("{}", e) }))).into_response(),
    }
}

async fn obtener_productos(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let categoria = params.get("categoria").cloned();
    let q = params.get("q").cloned();
    let result = match (categoria, q) {
        (Some(cat), Some(query)) => {
            let like = format!("%{}%", query.to_lowercase());
            sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos WHERE categoria = ? AND LOWER(referencia) LIKE ? ORDER BY fecha_venta DESC")
                .bind(cat).bind(like).fetch_all(&*state.db).await
        }
        (Some(cat), None) => {
            sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos WHERE categoria = ? ORDER BY fecha_venta DESC")
                .bind(cat).fetch_all(&*state.db).await
        }
        (None, Some(query)) => {
            let like = format!("%{}%", query.to_lowercase());
            sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos WHERE LOWER(referencia) LIKE ? ORDER BY fecha_venta DESC")
                .bind(like).fetch_all(&*state.db).await
        }
        (None, None) => {
            sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos ORDER BY fecha_venta DESC")
                .fetch_all(&*state.db).await
        }
    };
    match result {
        Ok(p) => Json(p).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("{}", e) }))).into_response(),
    }
}

async fn buscar_producto(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let q = params.get("q").unwrap_or(&"".to_string()).to_lowercase();
    let like = format!("%{}%", q);
    let result = sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos WHERE LOWER(referencia) LIKE ? ORDER BY fecha_venta DESC")
        .bind(like).fetch_all(&*state.db).await;
    match result {
        Ok(p) => Json(p).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("{}", e) }))).into_response(),
    }
}

async fn recomendar_productos(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let referencia = match params.get("ref") {
        Some(r) => r.clone(),
        None => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Falta ref" }))).into_response(),
    };
    let base = sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos WHERE referencia = ? LIMIT 1")
        .bind(&referencia).fetch_optional(&*state.db).await;
    let producto_base = match base {
        Ok(Some(p)) => p,
        _ => return (StatusCode::NOT_FOUND, Json(json!({ "error": "Producto no encontrado" }))).into_response(),
    };
    let rec = sqlx::query_as::<_, Producto>("SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos WHERE categoria = ? AND referencia != ? ORDER BY cantidad DESC LIMIT 5")
        .bind(&producto_base.categoria).bind(&producto_base.referencia).fetch_all(&*state.db).await.unwrap_or_default();
    Json(rec).into_response()
}

// =======================
// Checkout PROFESIONAL - Wompi real
// =======================
async fn crear_checkout(State(state): State<AppState>, Json(payload): Json<CheckoutRequest>) -> impl IntoResponse {
    if payload.items.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "Carrito vacío" }))).into_response();
    }
    let reference = format!("ORD-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());
    let amount_in_cents = (payload.total * 100.0) as i64;
    let checkout_url = format!(
        "https://checkout.wompi.co/p/?public-key={}&currency={}&amount-in-cents={}&reference={}",
        state.wompi_public_key, payload.currency, amount_in_cents, reference
    );
    (StatusCode::OK, Json(CheckoutResponse { success: true, checkout_url, reference })).into_response()
}

// =======================
// Chatbot
// =======================
async fn chatbot(Json(payload): Json<MensajeUsuario>) -> Json<serde_json::Value> {
    let mensaje = payload.mensaje.to_lowercase();
    if mensaje.contains("hola") || mensaje.contains("buenas") {
        Json(json!({ "respuesta": "¡Hola! Soy de MontiTech 📸 ¿Buscas cámara digital retro?" }))
    } else if mensaje.contains("forro") || mensaje.contains("estuche") {
        Json(json!({ "respuesta": "Tenemos estuches. Escríbenos por WhatsApp con el modelo." }))
    } else if mensaje.contains("camara") || mensaje.contains("cámara") {
        Json(json!({ "respuesta": "Tenemos Sony W630, Samsung, Canon desde $399.999. ¿Cuál te interesa?" }))
    } else {
        Json(json!({ "respuesta": "Escríbenos por WhatsApp y te ayudamos con tu cámara ideal." }))
    }
}

// =======================
// Banners
// =======================
async fn obtener_banners(State(state): State<AppState>) -> Json<Vec<Banner>> {
    let banners = sqlx::query_as::<_, Banner>("SELECT id, nombre, referencia, costo, archivo_imagen, video_url, button_text, clicks, activo, orden FROM banners WHERE activo = 1 ORDER BY orden ASC, id DESC")
        .fetch_all(&*state.db).await.unwrap_or_default();
    Json(banners)
}

async fn click_banner(Path(id): Path<i32>, State(state): State<AppState>) -> impl IntoResponse {
    let _ = sqlx::query("UPDATE banners SET clicks = clicks + 1 WHERE id = ?").bind(id).execute(&*state.db).await;
    StatusCode::OK
}

// =======================
// DB download
// =======================
async fn descargar_db() -> impl IntoResponse {
    match fs::read("db.sqlite") {
        Ok(content) => Response::builder().header("Content-Type", "application/octet-stream").header("Content-Disposition", "attachment; filename=db.sqlite").body(Body::from(content)).unwrap(),
        Err(_) => Response::builder().status(500).body(Body::from("Error al leer")).unwrap(),
    }
}

async fn save_multipart_file(mut multipart: Multipart, output_dir: &str, base_url: &str, url_prefix: &str) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    let mut urls = Vec::new();
    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": format!("{}", e) }))))? {
        let original_name = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| "archivo".to_string());
        let data = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": format!("{}", e) }))))?;
        let ext = std::path::Path::new(&original_name).extension().and_then(|s| s.to_str()).unwrap_or("jpg");
        let unique_name = format!("{}_{}.{}", Uuid::new_v4(), "file", ext);
        let ruta = format!("{}/{}", output_dir, unique_name);
        fs::write(&ruta, &data).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("{}", e) }))))?;
        urls.push(format!("{}/{}/{}", base_url.trim_end_matches('/'), url_prefix, unique_name));
    }
    Ok(urls)
}

async fn subir_banners(State(state): State<AppState>, multipart: Multipart) -> impl IntoResponse {
    match save_multipart_file(multipart, "./static/images", &state.base_url, "static/images").await {
        Ok(urls) => {
            for url in &urls {
                let nombre = url.split('/').last().unwrap_or("banner").to_string();
                let _ = sqlx::query("INSERT INTO banners (nombre, archivo_imagen, clicks) VALUES (?, ?, 0)").bind(&nombre).bind(&nombre).execute(&*state.db).await;
            }
            (StatusCode::OK, Json(json!({ "success": true, "urls": urls }))).into_response()
        }
        Err(e) => e.into_response(),
    }
}

async fn subir_imagen_producto(State(state): State<AppState>, multipart: Multipart) -> impl IntoResponse {
    match save_multipart_file(multipart, "./static/products", &state.base_url, "static/products").await {
        Ok(urls) => (StatusCode::OK, Json(json!({ "success": true, "urls": urls }))).into_response(),
        Err(e) => e.into_response(),
    }
}

// =======================
// Leads / Métricas
// =======================
async fn crear_lead(State(state): State<AppState>, Json(payload): Json<NuevoLead>) -> impl IntoResponse {
    let created_at = Utc::now().to_rfc3339();
    let _ = sqlx::query("INSERT INTO leads (nombre, telefono, ciudad, canal, producto_referencia, mensaje, estado, created_at) VALUES (?, ?, ?, ?, ?, ?, 'nuevo', ?)")
        .bind(&payload.nombre).bind(&payload.telefono).bind(&payload.ciudad).bind(&payload.canal).bind(&payload.producto_referencia).bind(&payload.mensaje).bind(&created_at)
        .execute(&*state.db).await;
    (StatusCode::OK, Json(json!({ "success": true }))).into_response()
}

async fn obtener_stock_bajo(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let umbral = params.get("umbral").and_then(|v| v.parse::<i32>().ok()).unwrap_or(5);
    let items = sqlx::query_as::<_, StockBajoItem>("SELECT referencia, categoria, precio, imagen, cantidad FROM productos WHERE cantidad <= ? ORDER BY cantidad ASC")
        .bind(umbral).fetch_all(&*state.db).await.unwrap_or_default();
    Json(items).into_response()
}

async fn obtener_metricas(State(state): State<AppState>) -> impl IntoResponse {
    let total_productos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM productos").fetch_one(&*state.db).await.unwrap_or(0);
    let valor_inventario: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(precio * cantidad), 0) FROM productos").fetch_one(&*state.db).await.unwrap_or(0.0);
    let total_leads: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM leads").fetch_one(&*state.db).await.unwrap_or(0);
    let today = Utc::now().date_naive().to_string();
    let leads_hoy: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM leads WHERE created_at LIKE ?").bind(format!("{}%", today)).fetch_one(&*state.db).await.unwrap_or(0);
    let banners_clicks_total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(clicks), 0) FROM banners").fetch_one(&*state.db).await.unwrap_or(0);
    let top = sqlx::query_as::<_, StockBajoItem>("SELECT referencia, categoria, precio, imagen, cantidad FROM productos ORDER BY cantidad ASC LIMIT 10").fetch_all(&*state.db).await.unwrap_or_default();
    Json(MetricasResponse { total_productos, valor_inventario, total_leads, leads_hoy, banners_clicks_total, top_productos_stock: top }).into_response()
}

async fn obtener_producto_imagenes(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let referencia = match params.get("ref") { Some(r) => r.clone(), None => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Falta ref" }))).into_response(), };
    let imgs = sqlx::query_as::<_, ProductoImagen>("SELECT id, producto_referencia, imagen_url, orden FROM producto_imagenes WHERE producto_referencia = ? ORDER BY orden ASC")
        .bind(referencia).fetch_all(&*state.db).await.unwrap_or_default();
    Json(imgs).into_response()
}

async fn crear_producto_imagen(State(state): State<AppState>, Json(payload): Json<NuevaProductoImagen>) -> impl IntoResponse {
    let _ = sqlx::query("INSERT INTO producto_imagenes (producto_referencia, imagen_url, orden) VALUES (?, ?, ?)")
        .bind(&payload.producto_referencia).bind(&payload.imagen_url).bind(payload.orden).execute(&*state.db).await;
    (StatusCode::OK, Json(json!({ "success": true }))).into_response()
}

async fn eliminar_producto_imagen(Path(id): Path<i64>, State(state): State<AppState>) -> impl IntoResponse {
    let _ = sqlx::query("DELETE FROM producto_imagenes WHERE id = ?").bind(id).execute(&*state.db).await;
    (StatusCode::OK, Json(json!({ "success": true }))).into_response()
}
