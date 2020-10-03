use globset::{Glob, GlobSetBuilder};
use serde_derive::Deserialize;
use version_compare::{CompOp, Version};

use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;

use crate::{pkg, utils, NebulaError, Package, RepoType, Repository, CONFIG};

#[derive(Deserialize, Clone, Debug)]
pub struct NebulaConfig {
    pub repository: String,
}

const PKG_INDEX_NAME: &'static str = "packages.toml";

pub struct RepoNebula<'d> {
    repo_dir: PathBuf,
    conf: &'d NebulaConfig,
}

impl<'d> RepoNebula<'d> {
    pub fn new() -> Result<RepoNebula<'d>, NebulaError> {
        let conf = &CONFIG.repos().nebula();
        let repo_dir = CONFIG.repos_dir().join("nebula");
        Ok(RepoNebula { conf, repo_dir })
    }
}

impl<'d> Repository for RepoNebula<'d> {
    fn initialize(&self) -> Result<(), NebulaError> {
        if !self.repo_dir.is_dir() {
            if let Err(e) = fs::create_dir(&self.repo_dir) {
                return Err(NebulaError::Fs(e.to_string()));
            }
        }
        Ok(())
    }

    fn repo_type(&self) -> RepoType {
        RepoType::Nebula
    }

    fn update(&self) -> Result<(), NebulaError> {
        // remove old files from debian/repo
        for entry in self.repo_dir.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
                if let Err(err) = fs::remove_file(entry.path()) {
                    return Err(NebulaError::Fs(format!(
                        "Cannot clean nebula repo file: {}",
                        err
                    )));
                }
            }
        }

        info!("Downloading nebula's packages file...");
        utils::download(
            format!("{}/{}", self.conf.repository, PKG_INDEX_NAME),
            &self.repo_dir.join(PKG_INDEX_NAME),
        );
        Ok(())
    }

    fn search(
        &self,
        queries: &[(&str, Option<(CompOp, Version)>)],
    ) -> Result<Vec<Vec<Package>>, NebulaError> {
        // create the glob set from queries
        let mut builder = GlobSetBuilder::new();
        for item in queries {
            builder.add(match Glob::new(format!("name: {}", item.0).as_str()) {
                Ok(g) => g,
                Err(e) => return Err(NebulaError::GlobError(e.to_string())),
            });
        }
        let glob_set = match builder.build() {
            Ok(g) => g,
            Err(e) => return Err(NebulaError::GlobError(e.to_string())),
        };

        // open file and store lines in a buffer
        let reader = match fs::File::open(format!("{}/{}", self.repo_dir.display(), PKG_INDEX_NAME))
        {
            Ok(r) => BufReader::new(r),
            Err(e) => return Err(NebulaError::Fs(e.to_string())),
        };
        let lines: Vec<String> = match reader.lines().collect() {
            Ok(s) => s,
            Err(e) => panic!("Fatal: {}", e),
        };

        let mut pkgs_list = vec![vec![]; queries.len()];
        let mut line_index = 0;
        while line_index < lines.len() {
            let line = lines[line_index].trim_end();
            if glob_set.is_match(&line) {
                let name = line[6..].to_string(); // pkg's name
                let mut match_indx = glob_set.matches(&line);
                let mut version = None; // must be some at the end of the scope
                let mut source = None; // must be some at the end of the scope
                let mut depends = None;
                line_index += 1;

                while line_index < lines.len() && !match_indx.is_empty() {
                    let line = lines[line_index].trim_end();
                    if line.is_empty() {
                        break; // reached end of package info
                    }
                    if line.contains("version: ") {
                        let capt_ver = match Version::from(&line[9..]) {
                            Some(v) => {
                                version = Some(line[9..].to_string());
                                v
                            }
                            None => return Err(NebulaError::NotSupportedVersion),
                        };
                        // check if the package continues to match the queries after verisons are
                        // compared
                        let mut i = 0;
                        while i < match_indx.len() {
                            if let Some((comp_op, comp_ver)) = &queries[match_indx[i]].1 {
                                if !capt_ver.compare_to(&comp_ver, &comp_op) {
                                    // if version req. is not satisfied
                                    match_indx.remove(i); // remove match from match indexes list
                                }
                            }
                            i += 1;
                        }
                    } else if line.contains("source: ") {
                        source = Some(pkg::PkgSource::from(
                            RepoType::Nebula,
                            Some(format!("{}/{}", self.conf.repository, &line[8..]).as_str()),
                        ));
                    } else if line.starts_with("depends: ") {
                        unimplemented!();
                    }
                    line_index += 1
                }
                // all info about the matched package has been read and parsed
                // check if the package still staisfies a a query
                if !match_indx.is_empty() {
                    if let Some(v) = version {
                        if let Some(src) = source {
                            for mat_i in match_indx {
                                pkgs_list[mat_i].push(Package::new(
                                    &name,
                                    &v,
                                    src.clone(),
                                    depends.clone(),
                                )?);
                            }
                        } else {
                            return Err(NebulaError::SourceParsingError);
                        }
                    } else {
                        return Err(NebulaError::VersionParsingError);
                    }
                }
            }
            line_index += 1;
        }
        Ok(pkgs_list)
    }
}
