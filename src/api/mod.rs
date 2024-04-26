use std::path::Path;

use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{
    get, post,
    web::{self, ServiceConfig},
    HttpResponse,
};
use serde::Deserialize;
use tokio::{
    fs::{self, File},
    io::{self},
};

use crate::pkg::{Platform, UpdatePkg};

mod api_response;

#[get("/")]
async fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Debug, MultipartForm)]
pub struct UploadPkg {
    project_name: Text<String>,
    pkg_version: Text<String>,
    pkg_link: Option<Text<String>>,
    file: Option<TempFile>,
    platform: Text<Platform>,
    update_log: Option<Text<String>>,
    mandatory: Text<bool>,
}

#[post("/upload")]
async fn upload(
    MultipartForm(form): MultipartForm<UploadPkg>,
    db: web::Data<mongodb::Database>,
) -> Result<HttpResponse, actix_web::Error> {
    let ext_err = Ok(api_response::err(500, "文件类型错误"));

    let Ok(pkg) = UpdatePkg::find_new_version(&form.project_name, &form.platform, &db).await else {
        return Ok(api_response::err(500, "查询失败"));
    };
    let version = if let Some(pkg) = pkg {
        if pkg.app_version > form.pkg_version.0 {
            return Ok(api_response::err(500, "已有更新的版本"));
        }
        pkg.version + 1
    } else {
        0
    };

    let (new_file_name, full_pkg) = if let Some(file) = form.file {
        let Some(file_name) = file.file_name else {
            return ext_err;
        };
        let Some(ext) = file_name.split('.').last() else {
            return ext_err;
        };
        if ext != "zip" && ext != "ipa" && ext != "apk" {
            return ext_err;
        }

        let path = Path::new("./pkgs");
        let Ok(has) = fs::try_exists(&path).await else {
            return Ok(api_response::err(500, "文件系统错误"));
        };
        if !has {
            fs::create_dir_all(&path).await?;
        }
        let new_file_name = format!(
            "{}_{}_{}.{}",
            form.project_name.0,
            form.pkg_version.0,
            chrono::Local::now().format("%Y%m%d%H%M%S"),
            ext
        );
        let new_path = path.join(&new_file_name);
        if cfg!(target_os = "windows") {
            let mut new_file = File::create(new_path).await?;
            let mut temp_file = File::from_std(file.file.into_file());
            io::copy(&mut temp_file, &mut new_file).await?;
        } else if let Err(err) = file.file.persist(new_path) {
            return Ok(api_response::err(500, err.to_string()));
        }
        (Some(new_file_name), ext != "zip")
    } else {
        (None, true)
    };

    let update_pkg = UpdatePkg {
        project_name: form.project_name.0,
        pkg_link: form.pkg_link.map(|v| v.0),
        pkg_file_name: new_file_name,
        app_version: form.pkg_version.0,
        version,
        mandatory: form.mandatory.0,
        update_log: form.update_log.map(|x| x.0),
        full_pkg,
        platform: form.platform.0,
    };
    if let Err(err) = update_pkg.save(&db).await {
        return Ok(api_response::err(500, err.to_string()));
    }
    Ok(api_response::ok(0))
}

#[derive(Debug, Deserialize)]
struct AppVersion {
    version: String,
    platform: Platform,
}

#[get("/remove/{project_name}")]
async fn remove(
    project_name: web::Path<String>,
    app_version: web::Query<AppVersion>,
    db: web::Data<mongodb::Database>,
) -> Result<HttpResponse, actix_web::Error> {
    log::info!("{} {:?}", project_name, app_version);
    let Ok(update_pkg) = UpdatePkg::find_by_version(
        &project_name.into_inner(),
        &app_version.platform,
        &app_version.version,
        &db,
    )
    .await
    else {
        return Ok(api_response::err(500, "查询失败"));
    };
    let Some(update_pkg) = update_pkg else {
        return Ok(api_response::err(500, "不存在"));
    };
    if let Err(err) = update_pkg.remove(&db).await {
        return Ok(api_response::err(500, err.to_string()));
    }
    Ok(api_response::ok(0))
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(index).service(upload).service(remove);
}
