use super::posts::Post;
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

static MANIFEST_FILE: &str = "blog.yml";
static POSTS_EXT: &str = "md";

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct Manifest {
    /// Title to display in the "top row".
    pub(crate) title: String,

    /// Title to use in the html header.
    pub(crate) index_title: String,

}

#[derive(Serialize)]
pub(crate) struct Blog {
    title: String,
    index_title: String,
    posts: Vec<Post>,
}

impl Blog {
    pub fn load(dir: &Path) -> Result<Self, Box<dyn Error>> {
        let manifest_content = std::fs::read_to_string(dir.join(MANIFEST_FILE))?;
        let manifest: Manifest = serde_yaml::from_str(&manifest_content)?;

        let mut posts = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if path.metadata()?.file_type().is_file() && ext == Some(POSTS_EXT) {
                posts.push(Post::open(&path)?);
            }
        }

        posts.sort_by_key(|post| {
            format!(
                "{}-{:02}-{:02}-{}",
                post.year, post.month, post.day, post.title
            )
        });
        posts.reverse();

        // Decide which posts should show the year in the index.
        posts[0].show_year = true;
        for i in 1..posts.len() {
            posts[i].show_year = posts[i - 1].year != posts[i].year;
        }

        // Make the updated time is unique, by incrementing seconds for duplicates
        let mut last_matching_updated = 0;
        for i in 1..posts.len() {
            if posts[i].updated == posts[last_matching_updated].updated {
                posts[i].set_updated((i - last_matching_updated) as u32);
            } else {
                last_matching_updated = i;
            }
        }

        Ok(Blog {
            title: manifest.title,
            index_title: manifest.index_title,
            posts,
        })
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn index_title(&self) -> &str {
        &self.index_title
    }

    pub(crate) fn path_back_to_root(&self) -> PathBuf {
        PathBuf::new().components().map(|_| Path::new("../")).collect()
    }

    pub(crate) fn posts(&self) -> &[Post] {
        &self.posts
    }
}