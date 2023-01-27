mod blogs;
mod posts;

use self::blogs::Blog;
use self::posts::Post;
use handlebars::{handlebars_helper, Handlebars};
use sass_rs::{compile_file, Options};
use serde_derive::Serialize;
use serde_json::json;
use std::convert::AsRef;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

struct Generator<'a> {
    handlebars: Handlebars<'a>,
    blog: Blog,
    out_directory: PathBuf,
}

#[derive(Debug, Serialize)]
struct Releases {
    releases: Vec<ReleasePost>,
    feed_updated: String,
}

#[derive(Debug, Serialize)]
struct ReleasePost {
    title: String,
    url: String,
}
handlebars_helper!(hb_month_name_helper: |month_num: u64| match month_num {
    1 => "Jan.",
    2 => "Feb.",
    3 => "Mar.",
    4 => "Apr.",
    5 => "May",
    6 => "June",
    7 => "July",
    8 => "Aug.",
    9 => "Sept.",
    10 => "Oct.",
    11 => "Nov.",
    12 => "Dec.",
    _ => "Error!",
});

impl<'a> Generator<'a> {
    fn new(
        out_directory: impl AsRef<Path>,
        posts_directory: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_templates_directory(".hbs", "templates")?;
        handlebars.register_helper("month_name", Box::new(hb_month_name_helper));

        Ok(Generator {
            handlebars,
            blog: Blog::load(posts_directory.as_ref())?,
            out_directory: out_directory.as_ref().into(),
        })
    }

    fn file_url(&self, path: &Path) -> String {
        format!(
            "file:///{}/{}",
            self.out_directory
                .canonicalize()
                .unwrap_or_else(|_| self.out_directory.to_owned())
                .display()
                .to_string()
                .trim_start_matches('/')
                .replace(' ', "%20")
                .replace("\\\\?\\", ""),
            path.display()
        )
        .replace(std::path::MAIN_SEPARATOR, "/")
    }

    fn render(&self) -> Result<(), Box<dyn Error>> {
        // make sure our output directory exists
        fs::create_dir_all(&self.out_directory)?;

        self.render_blog(&self.blog)?;
        self.compile_sass("app");
        self.compile_sass("fonts");
        self.concat_vendor_css(vec!["skeleton", "tachyons"]);
        self.copy_static_files()?;
        Ok(())
    }

    fn compile_sass(&self, filename: &str) {
        let scss_file = format!("./src/styles/{}.scss", filename);
        let css_file = format!("./static/styles/{}.css", filename);

        let css = compile_file(&scss_file, Options::default())
            .expect(&format!("couldn't compile sass: {}", &scss_file));
        let mut file =
            File::create(&css_file).expect(&format!("couldn't make css file: {}", &css_file));
        file.write_all(&css.into_bytes())
            .expect(&format!("couldn't write css file: {}", &css_file));
    }

    fn concat_vendor_css(&self, files: Vec<&str>) {
        let mut concatted = String::new();
        for filestem in files {
            let vendor_path = format!("./static/styles/{}.css", filestem);
            let contents = fs::read_to_string(vendor_path).expect("couldn't read vendor css");
            concatted.push_str(&contents);
        }
        fs::write("./static/styles/vendor.css", &concatted).expect("couldn't write vendor css");
    }

    fn render_blog(&self, blog: &Blog) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(self.out_directory.clone())?;

        let path = self.render_index(blog)?;

        println!("{}: {}", blog.title(), self.file_url(&path));

        for (i, post) in blog.posts().iter().enumerate() {
            let path = self.render_post(blog, post)?;
            if i == 0 {
                println!("└─ Latest post: {}\n", self.file_url(&path));
            }
        }

        Ok(())
    }

    fn render_index(&self, blog: &Blog) -> Result<PathBuf, Box<dyn Error>> {
        let data = json!({
            "title": blog.index_title(),
            "parent": "layout",
            "blog": blog,
            "root": blog.path_back_to_root(),
        });
        let path = PathBuf::from("index.html");
        self.render_template(&path, "index", data)?;
        Ok(path)
    }

    fn render_post(&self, blog: &Blog, post: &Post) -> Result<PathBuf, Box<dyn Error>> {
        let path = PathBuf::new()
            .join(format!("{:04}", &post.year))
            .join(format!("{:02}", &post.month))
            .join(format!("{:02}", &post.day));
        fs::create_dir_all(self.out_directory.join(&path))?;

        // then, we render the page in that path
        let mut filename = PathBuf::from(&post.filename);
        filename.set_extension("html");

        let data = json!({
            "title": format!("{} | {}", post.title, blog.title()),
            "parent": "layout",
            "blog": blog,
            "post": post,
            "root": blog.path_back_to_root().join("../../../"),
        });

        let path = path.join(filename);
        self.render_template(&path, "post", data)?;
        Ok(path)
    }

    fn copy_static_files(&self) -> Result<(), Box<dyn Error>> {
        use fs_extra::dir::{self, CopyOptions};

        let mut options = CopyOptions::new();
        options.overwrite = true;
        options.copy_inside = true;

        dir::copy("static/fonts", &self.out_directory, &options)?;
        dir::copy("static/images", &self.out_directory, &options)?;
        dir::copy("static/styles", &self.out_directory, &options)?;
        dir::copy("static/scripts", &self.out_directory, &options)?;

        Ok(())
    }

    fn render_template(
        &self,
        name: impl AsRef<Path>,
        template: &str,
        data: serde_json::Value,
    ) -> Result<(), Box<dyn Error>> {
        let out_file = self.out_directory.join(name.as_ref());
        let file = File::create(out_file)?;
        self.handlebars.render_to_write(template, &data, file)?;
        Ok(())
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let blog = Generator::new("site", "posts")?;

    blog.render()?;

    Ok(())
}
