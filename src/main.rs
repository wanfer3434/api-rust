use axum::{
    extract::{State, Query, Multipart},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqlitePool, FromRow};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;  // esto es necesario para generar nombres únicos
use std::{
    collections::HashMap,
    fs,
    sync::Arc,
};
use chrono::Utc;
use dotenv::dotenv;
use hyper::Method;
use std::fs::read;
use std::net::SocketAddr;

// =======================
// Estructuras de datos
// =======================

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
    archivo_imagen: String,
    video_url: Option<String>,      // 🔥 Puede ser null
    button_text: Option<String>,    // 🔥 Texto del botón, opcional
    clicks: i32,                    // 🔥 Contador de clicks
}

#[derive(Debug, Deserialize)]
struct MensajeUsuario {
    mensaje: String,
}

// =======================
// Estado compartido
// =======================

#[derive(Clone)]
struct AppState {
    db: Arc<SqlitePool>,
}

// =======================
// Punto de entrada
// =======================

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL no está definido en el entorno");

    println!("🟢 DB URL EN USO => {}", database_url);

    let pool = SqlitePoolOptions::new()
        .connect(&database_url)
        .await
        .expect("No se pudo conectar a la base de datos");

    let state = AppState {
        db: Arc::new(pool),
    };

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    // Rutas
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
	.route("/banners/click/{id}", post(click_banner))
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
// Handlers
// =======================

async fn root() -> &'static str {
    "¡Hola desde Rust y Axum!"
}

async fn saludo() -> Json<serde_json::Value> {
    Json(json!({ "mensaje": "Hola, bienvenido a mi API" }))
}

async fn crear_producto(
    State(state): State<AppState>,
    Json(mut producto): Json<Producto>,
) -> Json<serde_json::Value> {
    let fecha_actual = Utc::now().to_rfc3339();
    producto.fecha_venta = fecha_actual.clone();

    let resultado = sqlx::query(
        "INSERT INTO productos (referencia, categoria, precio, fecha_venta, imagen, cantidad)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&producto.referencia)
    .bind(&producto.categoria)
    .bind(&producto.precio)
    .bind(&producto.fecha_venta)
    .bind(&producto.imagen)
    .bind(&producto.cantidad)
    .execute(&*state.db)
    .await;

    match resultado {
        Ok(_) => Json(json!({ "mensaje": "Producto guardado correctamente" })),
        Err(err) => {
            eprintln!("❌ Error al guardar el producto: {}", err);
            Json(json!({ "error": "Error al guardar el producto" }))
        }
    }
}

async fn obtener_productos(State(state): State<AppState>) -> Json<Vec<Producto>> {
    let productos = sqlx::query_as::<_, Producto>(
        "SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad FROM productos",
    )
    .fetch_all(&*state.db)
    .await
    .unwrap_or_default();

    Json(productos)
}

async fn buscar_producto(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").unwrap_or(&"".to_string()).to_lowercase();
    let like_pattern = format!("%{}%", q);

    let result = sqlx::query_as::<_, Producto>(
        "SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
         FROM productos WHERE LOWER(referencia) LIKE ?",
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

async fn chatbot(
    axum::Json(payload): axum::Json<MensajeUsuario>,
) -> axum::Json<serde_json::Value> {
    let mensaje = payload.mensaje.to_lowercase();

    if mensaje.contains("abogado") || mensaje.contains("asesoría legal") {
        axum::Json(json!({
            "respuesta": "Puedes contactar al abogado Juan Guillermo Jiménez para tu asesoría legal."
        }))
    } else if mensaje.contains("hola") || mensaje.contains("buenas") {
        axum::Json(json!({
            "respuesta": "¡Hola! ¿En qué puedo ayudarte hoy?"
        }))
    } else {
        axum::Json(json!({
            "respuesta": "Lo siento, no entendí tu solicitud. ¿Podrías especificar mejor?"
        }))
    }
}

async fn obtener_banners(State(state): State<AppState>) -> Json<Vec<Banner>> {
    let banners = sqlx::query_as::<_, Banner>(
        "SELECT id, nombre, archivo_imagen, video_url, button_text, clicks  FROM banners ORDER BY RANDOM()",
    )
    .fetch_all(&*state.db)
    .await
    .unwrap_or_default();

    Json(banners)
}

// ============================================
// Función para registrar clicks en banners
// ============================================
async fn registrar_click(id: i32, db: &SqlitePool) {
    let result = sqlx::query!(
        "UPDATE banners SET clicks = clicks + 1 WHERE id = ?",
        id
    )
    .execute(db)
    .await;

    match result {
        Ok(_) => println!("✅ Click registrado para banner id {}", id),
        Err(err) => eprintln!("❌ Error al registrar click: {}", err),
    }
}

// Endpoint para recibir clicks desde Flutter
async fn click_banner(
    axum::extract::Path(id): axum::extract::Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    registrar_click(id, &state.db).await;
    StatusCode::OK
}
async fn descargar_db() -> impl IntoResponse {
    match read("db.sqlite") {
        Ok(content) => Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", "attachment; filename=db.sqlite")
            .body(axum::body::Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(500)
            .body(axum::body::Body::from("Error al leer el archivo"))
            .unwrap(),
    }
}

// =======================
// Upload de banners
// =======================

async fn subir_banners(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::Json<serde_json::Value> {
    let mut filenames = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        // Clonamos el nombre para no tener problemas de borrow
        let original_name = field.file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "imagen".to_string());

        // Leemos los bytes (consume field)
        let data = field.bytes().await.unwrap();

        // Nombre único
        let ext = std::path::Path::new(&original_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg");
        let unique_name = format!("{}_{}.{}", Uuid::new_v4(), "banner", ext);

        let ruta = format!("./static/images/{}", unique_name);

        fs::write(&ruta, &data).unwrap();

        sqlx::query!(
            "INSERT INTO banners (nombre, archivo_imagen) VALUES (?, ?)",
            original_name,
            unique_name
        )
        .execute(&*state.db)
        .await
        .unwrap();

        filenames.push(unique_name);
    }

    axum::Json(serde_json::json!({
        "success": true,
        "urls": filenames.iter().map(|f| format!("javier.tail33d395.ts.net/static/images/{}", f)).collect::<Vec<_>>()
    }))
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

    // 1. Obtener categoría del producto base
    let producto_base = sqlx::query_as::<_, Producto>(
    "SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
     FROM productos
     WHERE referencia = ?
     LIMIT 1",
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

    // 2. Buscar productos similares (ML simple: misma categoría + más vendidos)
    let recomendados = sqlx::query_as::<_, Producto>(
        "SELECT referencia, categoria, precio, fecha_venta, imagen, cantidad
         FROM productos
         WHERE categoria = ?
           AND referencia != ?
         ORDER BY cantidad DESC
         LIMIT 5",
    )
    .bind(&producto_base.categoria)
    .bind(&producto_base.referencia)
    .fetch_all(&*state.db)
    .await
    .unwrap_or_default();

    Json(recomendados).into_response()
}
