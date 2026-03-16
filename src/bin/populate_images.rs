use sqlx::SqlitePool;
use std::fs;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Conexión a la DB
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = SqlitePool::connect(&database_url).await?;

    // Carpeta de imágenes
    let images_path = "./static/images";
    let entries = fs::read_dir(images_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                
                // Verificar si ya existe en la tabla productos
                let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM productos WHERE imagen LIKE ?")
                    .bind(format!("%{}%", filename))
                    .fetch_one(&pool)
                    .await?;
                
                if exists.0 == 0 {
                    // Insertar producto nuevo con URL completa
                    let url = format!(
                        "https://javier.tail33d395.ts.net/static/images/{}",
                        filename.replace(" ", "%20")
                    );

                    sqlx::query(
                        "INSERT INTO productos (referencia, categoria, precio, fecha_venta, imagen, cantidad) 
                         VALUES (?, ?, ?, DATE('now'), ?, ?)"
                    )
                    .bind(filename)        // referencia = nombre del archivo
                    .bind("Desconocida")   // categoria por defecto
                    .bind(0.0_f64)         // precio por defecto
                    .bind(url)             // imagen = URL completa
                    .bind(1_i64)           // cantidad por defecto
                    .execute(&pool)
                    .await?;

                    println!("✅ Agregado: {}", filename);
                } else {
                    println!("⚠️ Ya existe: {}", filename);
                }
            }
        }
    }

    println!("✔️ Proceso terminado");
    Ok(())
}

