// @flow strict
import {
  MdEdit,
  MdFileOpen,
  MdFolder,
  MdAdd,
  MdMoreHoriz,
  MdSystemUpdateAlt,
  MdRefresh,
  MdOutlineInfo,
  MdSearch,
} from "react-icons/md";
import { createWindow, openFile, browserSupportedFiles } from "./actions";
import { ErrorBody, Folder, Tab } from "../types/definitions";
import { useState, useContext, useEffect } from "react";
import { GlobalContext } from "../context";
import { IoBugOutline } from "react-icons/io5";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Props = {
  data: {
    folders: Folder;
    error: ErrorBody;
    open: (url: string) => void;
    getIPs: (url: string) => void;
    updateTab: (name: string, path: string) => void;
    showSettings: boolean;
    openNewTab: () => void;
    openFolder: () => void;
    tabs: Tab[];
  };
};
function SideNav(props: Props) {
  const { API_URL, updateAnvel } = useContext(GlobalContext);
  const [searchView, setSearchView] = useState(false);
  const [moreStuff, setMoreStuff] = useState(false);
  const [searchError, setSearchError] = useState(<></>);
  const [searchResults, setSearchResults] = useState<Folder>({
    contents: [],
  });
  function handleSearch(e: React.ChangeEvent<HTMLInputElement>) {
    e.preventDefault();
    const input = e.target.value;
    const results: Folder = {
      contents: [],
    };
    props.data.folders.contents.forEach((content) => {
      if (content.name.includes(input)) {
        results.contents.push(content);
      } else {
        setSearchError(
          <>
            <p>
              Cannot found "
              {input.length > 9 ? <>{input.slice(0, 7)}...</> : input}"
            </p>
            <p className="font-semibold mt-2">Note :</p>
            <ul style={{ listStyleType: "initial", marginLeft: 20 }}>
              <li>Try searching with keywords.</li>
              <li>Consider case sensitive words.</li>
            </ul>
          </>,
        );
      }
    });
    setSearchResults(results);
  }

  const data = [
    {
      name: "Desktop",
    },
    {
      name: "Documents",
    },
    {
      name: "Downloads",
    },
    {
      name: "Music",
    },
    {
      name: "Pictures",
    },
    {
      name: "Videos",
    },
  ];

  useEffect(() => {
    updateAnvel();
  }, [updateAnvel]);

  return (
    <div
      id="sidebar"
      className="overflow-hidden bg-[var(--primary-02)] border-dotted border-[#3c3c3c]/50 border-r-[1px] h-[100vh] fixed pb-[12px] bottom-[18px] left-0 w-[200px] top-[35px] text-[13px]"
    >
      {!props.data.showSettings ? (
        <div className="flex flex-col mb-3">
          <div className="h-[46px] flex items-center uppercase pl-[12px] pr-[8px]">
            <button
              onClick={() => {
                setSearchResults({
                  contents: [],
                });
                setSearchError(<></>);
                setSearchView(false);
              }}
              title="File explorer"
              className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer  p-[4px]"
            >
              <MdFileOpen className="w-[18px] h-[18px]" />
            </button>
            <button
              title="Search for a folder or file"
              onClick={() => setSearchView(true)}
              className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer p-[4px]"
            >
              <MdSearch className="w-[18px] h-[18px]" />
            </button>
            <button
              title="Refresh tab"
              onClick={() => {
                props.data.showSettings === false
                  ? props.data.open(`${API_URL}/api/directory_content`)
                  : props.data.getIPs(`${API_URL}/api/get_ip_address`);
              }}
              className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer p-[4px]"
            >
              <MdRefresh className="w-[18px] h-[18px]" />
            </button>
            {props.data.tabs.length < 4 ? (
              <button
                title="Open a new tab"
                onClick={props.data.openNewTab}
                className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer p-[4px]"
              >
                <MdAdd className="w-[18px] h-[18px]" />
              </button>
            ) : (
              ""
            )}
          </div>
          {/* folders */}
          {searchView ? (
            ""
          ) : (
            <div className="resize-y">
              <div className="flex items-center text-[11px] uppercase px-[8px] h-[35px]">
                <p className="pl-[12px]">
                  {!moreStuff ? "EXPLORER" : "QUICK ACCESS"}
                </p>
                <button
                  onClick={() => {
                    if (!moreStuff) {
                      setMoreStuff(true);
                    } else {
                      setMoreStuff(false);
                    }
                  }}
                  className="w-[30px] ml-auto h-[25px] cursor-pointer p-[4px]"
                >
                  <MdMoreHoriz className="text-lg" />
                </button>
              </div>
              <div
                id="folders"
                className="sidebar_folders overflow-y-auto pb-[33px] pt-1 h-screen"
              >
                {!moreStuff ? (
                  <div className="flex flex-col">
                    {props.data.folders ? (
                      props.data.folders.contents.map((content) => {
                        let path = content.path;
                        if (path.includes("\\")) {
                          // Replace backslashes with forward slashes
                          path = path.replace(/\\/g, "/");
                        }

                        const unique_media = ["MP3", "MP4"];
                        let label = unique_media.includes(
                          content.metadata.file_extension.toUpperCase(),
                        )
                          ? content.metadata.file_extension
                          : content.name;
                        if (label.includes(" ") || label.includes(".")) {
                          label = label.replace(/ /g, "_");
                          if (label.includes(".")) {
                            label = label.replace(/\./g, "_");
                          }
                        }
                        return (
                          <div
                            className="flex-grow"
                            key={content.name}
                            title={content.name}
                          >
                            {content.metadata.is_file ? (
                              <button
                                key={content.name}
                                onClick={() => {
                                  if (!content.metadata.is_file) {
                                    props.data.updateTab(content.name, path);
                                    props.data.open(
                                      `${API_URL}/api/directory_content`,
                                    );
                                  } else {
                                    if (
                                      browserSupportedFiles(
                                        content.metadata.file_extension,
                                      )
                                    ) {
                                      if (path.includes("#"))
                                        path = path.replace(/#/g, "%23");
                                      createWindow(
                                        `file://${path}`,
                                        label,
                                        content.name,
                                      );
                                    } else {
                                      openFile(`${API_URL}/api/open`, path);
                                    }
                                  }
                                }}
                                className="flex w-[195px] items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300"
                              >
                                <MdFileOpen className="w-[20px] h-[20px] pr-[3px]" />
                                <p className="text-[11px] uppercase">
                                  {content.name.length < 20 ? (
                                    content.name
                                  ) : (
                                    <>{content.name.slice(0, 18)}...</>
                                  )}
                                </p>
                              </button>
                            ) : (
                              <button
                                onClick={() => {
                                  props.data.updateTab(content.name, path);
                                  props.data.open(
                                    `${API_URL}/api/directory_content`,
                                  );
                                }}
                                key={content.name}
                                className="flex w-[195px] flex-grow items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300"
                              >
                                <MdFolder className="w-[20px] h-[20px] pr-[3px]" />
                                <p className="text-[11px] uppercase">
                                  {content.name.length < 20 ? (
                                    content.name
                                  ) : (
                                    <>{content.name.slice(0, 18)}...</>
                                  )}
                                </p>
                              </button>
                            )}
                          </div>
                        );
                      })
                    ) : (
                      <div className="flex flex-col justify-start items-start py-2 px-3">
                        <p>{props.data.error.message}</p>
                        <button
                          onClick={props.data.openFolder}
                          className="mt-2 underline flex gap-2 text-blue-500 items-center justify-center"
                        >
                          <MdEdit className="w-[16px] h-[16px]" />
                          <span>Edit path</span>
                        </button>
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="flex flex-col">
                    {data.map((i) => {
                      return (
                        <div className="flex-grow" key={i.name}>
                          <button
                            onClick={() => {
                              //const path=localStorage.getItem("path")
                              //const accessPath=`${path.slice(0,path.lastIndexOf("/")+1)}${i.name}`
                              props.data.updateTab(i.name, i.name);
                              props.data.open(
                                `${API_URL}/api/directory_content`,
                              );
                            }}
                            className="flex w-[195px] items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300"
                          >
                            <MdFolder className="w-[20px] h-[20px] pr-[3px]" />
                            <p className="text-[11px] uppercase">{i.name}</p>
                          </button>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* search */}
          {searchView ? (
            <div id="search" className="resize-y">
              <div className="flex items-center text-[11px] uppercase px-[8px] h-[35px]">
                <input
                  onChange={handleSearch}
                  type="text"
                  className="px-2 py-[3px] border-[1px] border-violet-300 text-[12px] w-full placeholder:text-gray-800 bg-[var(--primary-02)] focus:outline-[1px] focus:border-none focus:outline-none focus:outline-violet-300"
                  placeholder="Search..."
                />
              </div>
              <div
                id="folders"
                className="sidebar_folders overflow-y-auto pb-[33px] pt-1 h-screen"
              >
                <div className="flex flex-col">
                  {searchResults.contents.length !== 0 ? (
                    searchResults.contents.map((content) => {
                      let path = content.path;
                      if (path.includes("\\")) {
                        // Replace backslashes with forward slashes
                        path = path.replace(/\\/g, "/");
                      }

                      const unique_media = ["MP3", "MP4"];
                      let label = unique_media.includes(
                        content.metadata.file_extension.toUpperCase(),
                      )
                        ? content.metadata.file_extension
                        : content.name;
                      if (label.includes(" ") || label.includes(".")) {
                        label = label.replace(/ /g, "_");
                        if (label.includes(".")) {
                          label = label.replace(/\./g, "_");
                        }
                      }
                      return (
                        <div
                          className="flex-grow"
                          key={content.name}
                          title={content.name}
                        >
                          {content.metadata.is_file ? (
                            <button
                              key={content.name}
                              onClick={() => {
                                if (!content.metadata.is_file) {
                                  props.data.updateTab(content.name, path);
                                  localStorage.setItem("path", path);
                                  props.data.open(
                                    `${API_URL}/api/directory_content`,
                                  );
                                } else {
                                  if (
                                    browserSupportedFiles(
                                      content.metadata.file_extension,
                                    )
                                  ) {
                                    if (path.includes("#"))
                                      path = path.replace(/#/g, "%23");
                                    createWindow(
                                      `file://${path}`,
                                      label,
                                      content.name,
                                    );
                                  } else {
                                    openFile(`${API_URL}/api/open`, path);
                                  }
                                }
                              }}
                              className="flex w-[195px] flex-grow items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300"
                            >
                              <MdFileOpen className="w-[20px] h-[20px] pr-[3px]" />
                              <p className="text-[11px] uppercase">
                                {content.name.length < 20 ? (
                                  content.name
                                ) : (
                                  <>{content.name.slice(0, 18)}...</>
                                )}
                              </p>
                            </button>
                          ) : (
                            <button
                              onClick={() => {
                                props.data.updateTab(content.name, path);
                                props.data.open(
                                  `${API_URL}/api/directory_content`,
                                );
                              }}
                              key={content.name}
                              id="folders_{name_str}"
                              className="flex w-[195px] flex-grow items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300"
                            >
                              <MdFolder className="w-[20px] h-[20px] pr-[3px]" />
                              <p className="text-[11px] uppercase">
                                {content.name.length < 20 ? (
                                  content.name
                                ) : (
                                  <>{content.name.slice(0, 18)}...</>
                                )}
                              </p>
                            </button>
                          )}
                        </div>
                      );
                    })
                  ) : (
                    <div className="flex flex-col justify-start items-start py-2 px-3">
                      <p>{searchError}</p>
                    </div>
                  )}
                </div>
              </div>
            </div>
          ) : (
            ""
          )}
        </div>
      ) : (
        <div className="flex flex-col mb-3">
          {/*when settings tabs is open then sidenav item would be the following*/}
          <div className="resize-y">
            <div className="sidebar_folders overflow-y-auto pt-[35px] pb-[33px] text-[14px] h-screen">
              <button className="flex h-[40px] items-center hover:bg-[var(--primary-05)] active:bg-[var(--primary-05)] px-[12px] cursor-default w-full">
                <MdOutlineInfo className="w-[21px] h-[23px] pr-[6px]" />
                <p>About</p>
              </button>
              <button
                id="update_anvel"
                onClick={async () => {
                  try {
                    const update = await check();
                    if (update) {
                      // Install the update. This will also restart the app on Windows!
                      await update.downloadAndInstall();

                      // On macOS and Linux you will need to restart the app manually.
                      // You could use this step to display another confirmation dialog.
                      await relaunch();
                    }
                  } catch (error: unknown) {
                    console.log(error);
                  }
                }}
                className="flex h-[40px] items-center hover:bg-[var(--primary-05)] active:bg-[var(--primary-05)] px-[12px] cursor-default w-full"
              >
                <MdSystemUpdateAlt className="w-[21px] h-[23px] pr-[6px]" />
                <p>Update Anvel</p>
              </button>

              <button
                onClick={() => {
                  window.location.href =
                    "mailto:imranmat254@gmail.com?subject=Reporting a bug in Anvel";
                }}
                className="flex h-[40px] items-center hover:bg-[var(--primary-05)] active:bg-[var(--primary-05)] px-[12px] cursor-default w-full"
              >
                <IoBugOutline className="w-[21px] h-[23px] pr-[6px]" />
                <p>Report a bug</p>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default SideNav;
