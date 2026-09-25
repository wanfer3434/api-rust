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
use sqlx::{FromRow, Row, SqlitePool};
use sqlx::sqlite::SqlitePoolOptions;

// =======================
// UTILS / GENERAL
// =======================
use uuid::Uuid;
use chrono::Utc;
use dotenv::dotenv;

use std::{
    collections::HashMap,
    env,
    fs,
    net::SocketAddr,
    sync::Arc,
    path::Path as StdPath,
};

// =======================
// HTTP CLIENT
// =======================

// =======================
// CRYPTO (HMAC / SHA)
// =======================
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
struct CheckoutRequest {
    items: Vec<CheckoutItem>,
    total: f64,
    currency: String,
    customer_email: Option<String>,
    customer_name: Option<String>,
    customer_phone: Option<String>,
    payment_method: Option<String>, // "nequi", "pse", "card"
}

#[derive(Debug, Serialize)]
struct CheckoutResponse {
    success: bool,
    checkout_url: String,
    reference: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct BalanceSnapshot {
    id: i64,
    asset: String,
    free: f64,
    locked: f64,
    captured_at: String,
}

// =======================
// Estado compartido
// =======================
#[derive(Clone)]
struct AppState {
    db: Arc<SqlitePool>,
    base_url: String,
    admin_token: String,
    wompi_public_key: String,
    wompi_private_key: String,
    wompi_integrity_key: String,
}
// =======================
// Main
// =======================

#[tokio::main]
async fn main() {
    dotenv::from_path("/home/javier/api-rust/.env").ok();
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL no está definido en el entorno");

    let base_url =
        env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let admin_token =
        env::var("ADMIN_TOKEN").unwrap_or_else(|_| "cambia-esto-por-un-token-seguro".to_string());

    let wompi_public_key =
        env::var("WOMPI_PUBLIC_KEY").unwrap_or_else(|_| "".to_string());

    let wompi_private_key =
        env::var("WOMPI_PRIVATE_KEY").unwrap_or_else(|_| "".to_string());

    let wompi_integrity_key =
        env::var("WOMPI_INTEGRITY_KEY").unwrap_or_else(|_| "".to_string());
    println!("🟢 DB URL EN USO => {}", database_url);
    println!("🟢 BASE_URL => {}", base_url);

    fs::create_dir_all("./static/images").expect("No se pudo crear ./static/images");
    fs::create_dir_all("./static/products").expect("No se pudo crear ./static/products");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("No se pudo conectar a la base de datos");

    let state = AppState {
        db: Arc::new(pool),
        base_url,
        admin_token,
        wompi_public_key,
        wompi_private_key,
        wompi_integrity_key,
    };
    let cors = CorsLayer::new()
        .allow_origin(Any)
    	.allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
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
	.route("/api/health", get(api_health))
        .nest_service("/static", ServeDir::new("./static"))
        .with_state(state.clone())
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Servidor corriendo en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("No se pudo enlazar el puerto");

    axum::serve(listener, app)
        .await
        .expect("Error al iniciar el servidor");
}	

// =======================
// Handlers básicos
// =======================
fn validate_admin(headers: &HeaderMap, expected_token: &str) -> Result<(), Response> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {}", expected_token);

    if auth_header != expected {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "No autorizado" })),
        )
            .into_response());
    }

    Ok(())
}

fn sanitize_filename(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::new();

    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch == ' ' || ch == '-' || ch == '_' {
            out.push('_');
        }
    }

    while out.contains("__") {
        out = out.replace("__", "_");
    }

    out.trim_matches('_').to_string()
}

fn normalize_extension(ext: &str) -> &str {
    match ext.to_lowercase().as_str() {
        "jpeg" => "jpg",
        "jpg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        _ => "jpg",
    }
}


async fn root() -> &'static str {
    "¡Hola desde Rust y Axum!"
}

async fn saludo() -> Json<serde_json::Value> {
    Json(json!({ "mensaje": "Hola, bienvenido a mi API" }))
}
//======================================
// endpoin para subir imagen del banner
//=====================================

async fn subir_banner_imagen_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) {
        return resp;
    }

    let next = multipart.next_field().await;

    let field = match next {
        Ok(Some(field)) => field,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "No se recibió ningún archivo en el multipart" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Error parsing multipart: {}", e) })),
            )
                .into_response();
        }
    };

    let original_name = field
        .file_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "banner.jpg".to_string());

    let data = match field.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("No se pudo leer archivo: {}", e) })),
            )
                .into_response();
        }
    };

    if data.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "El archivo llegó vacío" })),
        )
            .into_response();
    }

    let stem = StdPath::new(&original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("banner");

    let ext = StdPath::new(&original_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg");

    let ext = normalize_extension(ext);
    let clean_stem = sanitize_filename(stem);
    let unique_name = format!("{}_{}.{}", clean_stem, Uuid::new_v4(), ext);
    let ruta = format!("./static/images/{}", unique_name);

    if let Err(e) = fs::write(&ruta, &data) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("No se pudo guardar imagen: {}", e) })),
        )
            .into_response();
    }

    let image_url = format!(
        "{}/static/images/{}",
        state.base_url.trim_end_matches('/'),
        unique_name
    );

    (
        StatusCode::OK,
        Json(UploadBannerImageResponse {
            success: true,
            archivo_imagen: unique_name,
            image_url,
        }),
    )
        .into_response()
}
//=======================
// ENDPOIND PARA ELIMINAR PRODUCTO DE BANNER
//=========================================
async fn eliminar_banner_admin(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) {
        return resp;
    }

    let result = sqlx::query("DELETE FROM banners WHERE id = ?")
        .bind(id)
        .execute(&*state.db)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "mensaje": "Banner eliminado correctamente"
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error eliminando banner: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo eliminar el banner" })),
            )
                .into_response()
        }
    }
}

//======================
//ENDPOINT PARA CREAR BANNER
//=========================
async fn crear_banner_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BannerInput>,
) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) {
        return resp;
    }

    let result = sqlx::query(
        r#"
        INSERT INTO banners
        (nombre, referencia, costo, archivo_imagen, video_url, button_text, clicks, activo, orden)
        VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)
        "#,
    )
    .bind(&payload.nombre)
    .bind(&payload.referencia)
    .bind(payload.costo)
    .bind(&payload.archivo_imagen)
    .bind(&payload.video_url)
    .bind(payload.button_text.as_deref().unwrap_or("Ver demostración"))
    .bind(payload.activo)
    .bind(payload.orden)
    .execute(&*state.db)
    .await;

    match result {
        Ok(r) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "id": r.last_insert_rowid(),
                "mensaje": "Banner creado correctamente"
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error creando banner: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo crear el banner" })),
            )
                .into_response()
        }
    }
}

//======================
// ENDPOINT PARA ACTUALIZAR BANNER
//===============================
async fn actualizar_banner_admin(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BannerInput>,
) -> impl IntoResponse {
    if let Err(resp) = validate_admin(&headers, &state.admin_token) {
        return resp;
    }

    let result = sqlx::query(
        r#"
        UPDATE banners
        SET nombre = ?,
            referencia = ?,
            costo = ?,
            archivo_imagen = ?,
            video_url = ?,
            button_text = ?,
            activo = ?,
            orden = ?
        WHERE id = ?
        "#,
    )
    .bind(&payload.nombre)
    .bind(&payload.referencia)
    .bind(payload.costo)
    .bind(&payload.archivo_imagen)
    .bind(&payload.video_url)
    .bind(payload.button_text.as_deref().unwrap_or("Ver demostración"))
    .bind(payload.activo)
    .bind(payload.orden)
    .bind(id)
    .execute(&*state.db)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "mensaje": "Banner actualizado correctamente"
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error actualizando banner: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo actualizar el banner" })),
            )
                .into_response()
        }
    }
}


// =======================
// Productos
// =======================

async fn crear_producto(
    State(state): State<AppState>,
    Json(producto): Json<NuevoProducto>,
) -> impl IntoResponse {
    let fecha_actual = Utc::now().to_rfc3339();

    let resultado = sqlx::query(
        r#"
        INSERT OR REPLACE INTO productos
        (referencia, categoria, precio, fecha_venta, imagen, cantidad)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&producto.referencia)
    .bind(&producto.categoria)
    .bind(producto.precio)
    .bind(&fecha_actual)
    .bind(&producto.imagen)
    .bind(producto.cantidad)
    .execute(&*state.db)
    .await;

    match resultado {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "mensaje": "Producto guardado correctamente",
                "referencia": producto.referencia
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error al guardar el producto: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Error al guardar el producto" })),
            )
                .into_response()
        }
    }
}

async fn obtener_productos(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let categoria = params.get("categoria").cloned();
    let q = params.get("q").cloned();

    let result = match (categoria, q) {
        (Some(cat), Some(query)) => {
            let like_pattern = format!("%{}%", query.to_lowercase());
            sqlx::query_as::<_, Producto>(
                r#"
                SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
                FROM productos
                WHERE categoria = ?
                  AND LOWER(referencia) LIKE ?
                ORDER BY fecha_venta DESC
                "#,
            )
            .bind(cat)
            .bind(like_pattern)
            .fetch_all(&*state.db)
            .await
        }
        (Some(cat), None) => {
            sqlx::query_as::<_, Producto>(
                r#"
                SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
                FROM productos
                WHERE categoria = ?
                ORDER BY fecha_venta DESC
                "#,
            )
            .bind(cat)
            .fetch_all(&*state.db)
            .await
        }
        (None, Some(query)) => {
            let like_pattern = format!("%{}%", query.to_lowercase());
            sqlx::query_as::<_, Producto>(
                r#"
                SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
                FROM productos
                WHERE LOWER(referencia) LIKE ?
                ORDER BY fecha_venta DESC
                "#,
            )
            .bind(like_pattern)
            .fetch_all(&*state.db)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, Producto>(
                r#"
                SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
                FROM productos
                ORDER BY fecha_venta DESC
                "#,
            )
            .fetch_all(&*state.db)
            .await
        }
    };

    match result {
        Ok(productos) => Json(productos).into_response(),
        Err(e) => {
            eprintln!("❌ Error al obtener productos: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Error interno al obtener productos" })),
            )
                .into_response()
        }
    }
}

///======================
//Handler de checkout
///======================
async fn crear_checkout(
    State(_state): State<AppState>,
    Json(payload): Json<CheckoutRequest>,
) -> impl IntoResponse {
    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "El carrito está vacío"
            })),
        )
            .into_response();
    }

    if payload.total <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Total inválido"
            })),
        )
            .into_response();
    }

    let reference = format!("ORD-{}", Uuid::new_v4());

    // Por ahora simulamos una URL de checkout.
    // Aquí luego conectas Wompi / ePayco / PayU.
    let checkout_url = format!(
        "https://checkout.wompi.co/l/test-{}",
        reference
    );

    (
        StatusCode::OK,
        Json(CheckoutResponse {
            success: true,
            checkout_url,
            reference,
        }),
    )
        .into_response()
}

async fn buscar_producto(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").unwrap_or(&"".to_string()).to_lowercase();
    let like_pattern = format!("%{}%", q);

    let result = sqlx::query_as::<_, Producto>(
        r#"
        SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
        FROM productos
        WHERE LOWER(referencia) LIKE ?
        ORDER BY fecha_venta DESC
        "#,
    )
    .bind(like_pattern)
    .fetch_all(&*state.db)
    .await;

    match result {
        Ok(productos) => Json(productos).into_response(),
        Err(e) => {
            eprintln!("❌ Error al buscar productos: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Error interno al buscar productos" })),
            )
                .into_response()
        }
    }
}

async fn recomendar_productos(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let referencia = match params.get("ref") {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Falta parámetro ref" })),
            )
                .into_response()
        }
    };

    let producto_base = sqlx::query_as::<_, Producto>(
        r#"
        SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
        FROM productos
        WHERE referencia = ?
        LIMIT 1
        "#,
    )
    .bind(referencia)
    .fetch_optional(&*state.db)
    .await;

    let producto_base = match producto_base {
        Ok(Some(p)) => p,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Producto no encontrado" })),
            )
                .into_response()
        }
    };

    let recomendados = sqlx::query_as::<_, Producto>(
        r#"
        SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
        FROM productos
        WHERE categoria = ?
          AND referencia != ?
        ORDER BY cantidad DESC
        LIMIT 5
        "#,
    )
    .bind(&producto_base.categoria)
    .bind(&producto_base.referencia)
    .fetch_all(&*state.db)
    .await
    .unwrap_or_default();

    Json(recomendados).into_response()
}

// =======================
// Chatbot
// =======================

async fn chatbot(Json(payload): Json<MensajeUsuario>) -> Json<serde_json::Value> {
    let mensaje = payload.mensaje.to_lowercase();

    if mensaje.contains("abogado") || mensaje.contains("asesoría legal") {
        Json(json!({
            "respuesta": "Puedes contactar al abogado Juan Guillermo Jiménez para tu asesoría legal."
        }))
    } else if mensaje.contains("hola") || mensaje.contains("buenas") {
        Json(json!({
            "respuesta": "¡Hola! ¿En qué puedo ayudarte hoy?"
        }))
    } else if mensaje.contains("forro") || mensaje.contains("estuche") {
        Json(json!({
            "respuesta": "Tenemos estuches disponibles. Escríbenos por WhatsApp con el modelo de tu celular."
        }))
    } else {
        Json(json!({
            "respuesta": "Lo siento, no entendí tu solicitud. ¿Podrías especificar mejor?"
        }))
    }
}

// =======================
// Banners
// =======================
async fn obtener_banners(State(state): State<AppState>) -> Json<Vec<Banner>> {
    let banners = sqlx::query_as::<_, Banner>(
        r#"
        SELECT
            id,
            nombre,
            referencia,
            costo,
            archivo_imagen,
            video_url,
            button_text,
            clicks,
            activo,
            orden
        FROM banners
        WHERE activo = 1
        ORDER BY orden ASC, id DESC
        "#,
    )
    .fetch_all(&*state.db)
    .await
    .unwrap_or_default();

    Json(banners)
}

async fn registrar_click(id: i32, db: &SqlitePool) {
    let result = sqlx::query("UPDATE banners SET clicks = clicks + 1 WHERE id = ?")
        .bind(id)
        .execute(db)
        .await;

    match result {
        Ok(_) => println!("✅ Click registrado para banner id {}", id),
        Err(err) => eprintln!("❌ Error al registrar click: {}", err),
    }
}

async fn click_banner(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    registrar_click(id, &state.db).await;
    StatusCode::OK
}

// =======================
// Descargar DB
// =======================

async fn descargar_db() -> impl IntoResponse {
    match fs::read("db.sqlite") {
        Ok(content) => Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", "attachment; filename=db.sqlite")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(500)
            .body(Body::from("Error al leer el archivo"))
            .unwrap(),
    }
}

// =======================
// Upload genérico
// =======================

async fn save_multipart_file(
    mut multipart: Multipart,
    output_dir: &str,
    base_url: &str,
    url_prefix: &str,
) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    let mut urls = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Error leyendo multipart: {}", e) })),
            )
        })?
    {
        let original_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "archivo".to_string());

        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Error leyendo bytes: {}", e) })),
            )
        })?;

        let ext = std::path::Path::new(&original_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}_{}.{}", Uuid::new_v4(), "file", ext);
        let ruta = format!("{}/{}", output_dir, unique_name);

        fs::write(&ruta, &data).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("No se pudo guardar el archivo: {}", e) })),
            )
        })?;

        let url = format!("{}/{}/{}", base_url.trim_end_matches('/'), url_prefix, unique_name);
        urls.push(url);
    }

    Ok(urls)
}

// =======================
// Upload banners
// =======================

async fn subir_banners(
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    let result = save_multipart_file(
        multipart,
        "./static/images",
        &state.base_url,
        "static/images",
    )
    .await;

    let urls = match result {
        Ok(urls) => urls,
        Err(err) => return err.into_response(),
    };

    for url in &urls {
        let nombre = url.split('/').last().unwrap_or("banner").to_string();
        let _ = sqlx::query(
            "INSERT INTO banners (nombre, archivo_imagen, clicks) VALUES (?, ?, 0)"
        )
        .bind(&nombre)
        .bind(&nombre)
        .execute(&*state.db)
        .await;
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "urls": urls
        })),
    )
        .into_response()
}

// =======================
// Upload imágenes de producto
// =======================

async fn subir_imagen_producto(
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    let result = save_multipart_file(
        multipart,
        "./static/products",
        &state.base_url,
        "static/products",
    )
    .await;

    match result {
        Ok(urls) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "urls": urls
            })),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

// =======================
// Leads
// =======================

async fn crear_lead(
    State(state): State<AppState>,
    Json(payload): Json<NuevoLead>,
) -> impl IntoResponse {
    let created_at = Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        INSERT INTO leads
        (nombre, telefono, ciudad, canal, producto_referencia, mensaje, estado, created_at)
        VALUES (?, ?, ?, ?, ?, ?, 'nuevo', ?)
        "#,
    )
    .bind(&payload.nombre)
    .bind(&payload.telefono)
    .bind(&payload.ciudad)
    .bind(&payload.canal)
    .bind(&payload.producto_referencia)
    .bind(&payload.mensaje)
    .bind(&created_at)
    .execute(&*state.db)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "mensaje": "Lead guardado correctamente"
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error al guardar lead: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo guardar el lead" })),
            )
                .into_response()
        }
    }
}

// =======================
// Stock bajo
// =======================

async fn obtener_stock_bajo(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let umbral = params
        .get("umbral")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(5);

    let result = sqlx::query_as::<_, StockBajoItem>(
        r#"
        SELECT referencia, categoria, precio, imagen, cantidad
        FROM productos
        WHERE cantidad <= ?
        ORDER BY cantidad ASC, referencia ASC
        "#,
    )
    .bind(umbral)
    .fetch_all(&*state.db)
    .await;

    match result {
        Ok(items) => Json(items).into_response(),
        Err(err) => {
            eprintln!("❌ Error consultando stock bajo: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo consultar stock bajo" })),
            )
                .into_response()
        }
    }
}

// =======================
// Métricas
// =======================

async fn obtener_metricas(State(state): State<AppState>) -> impl IntoResponse {
    let total_productos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM productos")
        .fetch_one(&*state.db)
        .await
        .unwrap_or(0);

    let valor_inventario: f64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(precio * cantidad), 0) FROM productos")
            .fetch_one(&*state.db)
            .await
            .unwrap_or(0.0);

    let total_leads: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM leads")
        .fetch_one(&*state.db)
        .await
        .unwrap_or(0);

    let today = Utc::now().date_naive().to_string();
    let like_today = format!("{}%", today);

    let leads_hoy: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM leads WHERE created_at LIKE ?")
            .bind(like_today)
            .fetch_one(&*state.db)
            .await
            .unwrap_or(0);

    let banners_clicks_total: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(clicks), 0) FROM banners")
            .fetch_one(&*state.db)
            .await
            .unwrap_or(0);

    let top_productos_stock = sqlx::query_as::<_, StockBajoItem>(
        r#"
        SELECT referencia, categoria, precio, imagen, cantidad
        FROM productos
        ORDER BY cantidad ASC, precio DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&*state.db)
    .await
    .unwrap_or_default();

    let resp = MetricasResponse {
        total_productos,
        valor_inventario,
        total_leads,
        leads_hoy,
        banners_clicks_total,
        top_productos_stock,
    };

    Json(resp).into_response()
}

// =======================
// Múltiples imágenes por producto
// =======================

async fn obtener_producto_imagenes(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let referencia = match params.get("ref") {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Falta parámetro ref" })),
            )
                .into_response()
        }
    };

    let result = sqlx::query_as::<_, ProductoImagen>(
        r#"
        SELECT id, producto_referencia, imagen_url, orden
        FROM producto_imagenes
        WHERE producto_referencia = ?
        ORDER BY orden ASC, id ASC
        "#,
    )
    .bind(referencia)
    .fetch_all(&*state.db)
    .await;

    match result {
        Ok(imagenes) => Json(imagenes).into_response(),
        Err(err) => {
            eprintln!("❌ Error obteniendo imágenes del producto: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudieron obtener las imágenes" })),
            )
                .into_response()
        }
    }
}

async fn crear_producto_imagen(
    State(state): State<AppState>,
    Json(payload): Json<NuevaProductoImagen>,
) -> impl IntoResponse {
    let result = sqlx::query(
        r#"
        INSERT INTO producto_imagenes
        (producto_referencia, imagen_url, orden)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(&payload.producto_referencia)
    .bind(&payload.imagen_url)
    .bind(payload.orden)
    .execute(&*state.db)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "mensaje": "Imagen adicional guardada correctamente"
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error guardando imagen adicional: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo guardar la imagen adicional" })),
            )
                .into_response()
        }
    }
}

async fn eliminar_producto_imagen(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM producto_imagenes WHERE id = ?")
        .bind(id)
        .execute(&*state.db)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "mensaje": "Imagen eliminada correctamente"
            })),
        )
            .into_response(),
        Err(err) => {
            eprintln!("❌ Error eliminando imagen: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "No se pudo eliminar la imagen" })),
            )
                .into_response()
        }
    }
}
//==============
//HELPER BINANCE
//==============
fn binance_base_url(mode: &str) -> &'static str {
    match mode {
        "mainnet" => "https://api.binance.com",
        _ => "https://testnet.binance.vision",
    }
}
fn sign_query(secret: &str, query: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC error");

    mac.update(query.as_bytes());
    let result = mac.finalize().into_bytes();

    hex::encode(result)
}
async fn binance_signed_post(
    state: &AppState,
    path: &str,
    extra_params: Vec<(&str, String)>,
) -> Result<serde_json::Value, String> {

    let client = Client::new();
    let timestamp = Utc::now().timestamp_millis();

    let mut params = vec![
        ("timestamp", timestamp.to_string()),
        ("recvWindow", "5000".to_string()),
    ];

    for item in extra_params {
        params.push(item);
    }

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let signature = sign_query(&state.binance_api_secret, &query);
    let body = format!("{}&signature={}", query, signature);

    let url = format!("{}{}", binance_base_url(&state.binance_mode), path);

    let response = client
        .post(url)
        .header("X-MBX-APIKEY", &state.binance_api_key)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(text);
    }

    serde_json::from_str(&text).map_err(|e| e.to_string())
}
async fn binance_signed_get(
    state: &AppState,
    path: &str,
    extra_params: Vec<(&str, String)>,
) -> Result<serde_json::Value, String> {

    let client = Client::new();
    let timestamp = Utc::now().timestamp_millis();

    let mut params = vec![
        ("timestamp", timestamp.to_string()),
        ("recvWindow", "5000".to_string()),
    ];

    for item in extra_params {
        params.push(item);
    }

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let signature = sign_query(&state.binance_api_secret, &query);

    let url = format!(
        "{}{}?{}&signature={}",
        binance_base_url(&state.binance_mode),
        path,
        query,
        signature
    );

    let response = client
        .get(url)
        .header("X-MBX-APIKEY", &state.binance_api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(text);
    }

    serde_json::from_str(&text).map_err(|e| e.to_string())
}
//===================
//binance_public_get
//==================
async fn binance_public_get(
    state: &AppState,
    path: &str,
    params: Vec<(&str, String)>,
) -> Result<serde_json::Value, String> {

    let client = Client::new();

    let query = if params.is_empty() {
        "".to_string()
    } else {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    };

    let url = if query.is_empty() {
        format!("{}{}", binance_base_url(&state.binance_mode), path)
    } else {
        format!("{}{}?{}", binance_base_url(&state.binance_mode), path, query)
    };

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(text);
    }

    serde_json::from_str(&text).map_err(|e| e.to_string())
}
