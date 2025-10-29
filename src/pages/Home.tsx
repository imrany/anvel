import { message } from "@tauri-apps/plugin-dialog";
import {
  MdArrowBack,
  MdClose,
  MdContentCopy,
  MdFolder,
  MdRefresh,
  MdInfoOutline,
  MdOpenInNew,
  MdSend,
  MdSettings,
} from "react-icons/md";
import Footer from "../components/Footer";
import SideNav from "../components/SideNav";
import ReportBugBtn from "../components/ReportBugBtn";
import AudioTag from "../components/AudioTag";
import TopNav from "../components/TopNav";
import { useContext, useEffect, useState } from "react";
import { GlobalContext } from "../context";
import {
  ErrorBody,
  Tab,
  Folder,
  Configurations,
  Content,
  Notifications,
  ChooseBackground,
  NetworkInformation,
  SendFileInfo,
} from "../types/definitions";
import {
  openFile,
  createWindow,
  browserSupportedFiles,
  openDialog,
} from "../components/actions";
import { useNavigate } from "react-router-dom";
import unknownFile from "../assets/icons/filetype/application-x-zerosize.svg";
import audioMp3 from "../assets/icons/filetype/audio-mp3.svg";
import videoMp4 from "../assets/icons/filetype/application-vnd.rn-realmedia.svg";
import videoWMV from "../assets/icons/filetype/video-x-ms-wmv.svg";
import audioWAV from "../assets/icons/filetype/audio-x-wav.svg";
import PDF from "../assets/icons/filetype/application-pdf.svg";
import DOCX from "../assets/icons/filetype/application-x-kword.svg";
import DOC from "../assets/icons/filetype/application-x-kword.svg";
import TXT from "../assets/icons/filetype/application-text.svg";
import ISOIMAGE from "../assets/icons/filetype/application-x-raw-disk-image.svg";
import PPTX from "../assets/icons/filetype/application-vnd.ms-powerpoint.svg";
import MKV from "../assets/icons/filetype/video-x-matroska.svg";
import AVI from "../assets/icons/filetype/video-x-wmv.svg";
import CSV from "../assets/icons/filetype/text-csv.svg";
import XLSX from "../assets/icons/filetype/libreoffice-oasis-spreadsheet.svg";
import PSD from "../assets/icons/filetype/image-vnd.adobe.photoshop.svg";
import DESKTOP from "../assets/icons/filetype/application-x-desktop.svg";
import OLD from "../assets/icons/filetype/application-x-trash.svg";
import ZIP from "../assets/icons/filetype/application-zip.svg";
import HTML from "../assets/icons/filetype/text-html.svg";
import CSS from "../assets/icons/filetype/text-css.svg";
import XML from "../assets/icons/filetype/text-xml.svg";
import PHP from "../assets/icons/filetype/application-x-php.svg";
import PYTHON from "../assets/icons/filetype/text-x-python.svg";
import SCRIPT from "../assets/icons/filetype/text-x-script.svg";
import JAVA from "../assets/icons/filetype/text-x-java.svg";
import CSHARP from "../assets/icons/filetype/text-csharp.svg";
import CPP from "../assets/icons/filetype/text-x-c++src.svg";
import PERL from "../assets/icons/filetype/application-x-perl.svg";
import CHEADER from "../assets/icons/filetype/text-x-chdr.svg";
import C from "../assets/icons/filetype/text-x-csrc.svg";
import RUBY from "../assets/icons/filetype/application-x-ruby.svg";
import EXE from "../assets/icons/filetype/application-octet-stream.svg";
import SYS from "../assets/icons/filetype/application-octet-stream.svg";
import APK from "../assets/icons/filetype/android-package-archive.svg";
import JS from "../assets/icons/filetype/application-javascript.svg";
import SQL from "../assets/icons/filetype/application-sql.svg";
import DEB from "../assets/icons/filetype/application-x-deb.svg";
import LNK from "../assets/icons/filetype/libreoffice-oasis-web-template.svg";
import FolderImage from "../assets/icons/folder.png";
import bg1 from "../assets/background/bg_1.png";
import { FileInfoDialog, OpenFolderDialog } from "../components/dialogs";
// import bg2 from "../assets/background/bg_2.png";
import indexedDb from "../components/indexedDb";

type Props = {
  data: {
    backgroundImage: string;
    changeBackground: (background: string) => void;
  };
};

export default function Home(props: Props) {
  const { API_URL } = useContext(GlobalContext);
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [audioSource, setAudioSource] = useState("");
  const [tabs, setTabs] = useState<Tab[]>([
    {
      name: "",
      createdAt: "",
      path: "",
      type: "",
      id: "",
    },
  ]);
  const [counter, setCounter] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [loadingText, setLoadingText] = useState("Loading...");
  const [isLoadingNetInfo, setIsLoadingNetInfo] = useState(true);
  const [isDisabled, setIsDisabled] = useState(false);
  const [backgroundOption, setBackgroundOption] = useState("");
  const [infoContent, setInfoContent] = useState<Content>({
    name: "",
    root: "",
    path: "",
    metadata: {
      is_file: false,
      file_extension: "",
    },
  });
  const [showSettings, setShowSettings] = useState(false);
  const [showSettingsTab, setShowSettingsTab] = useState(false);
  const [startRequestLoop, setStartRequestLoop] = useState(false);
  const [settingsHeader, setSettingsHeader] = useState("");
  const [networkInformation, setNetworkInformation] =
    useState<NetworkInformation>({
      internal: "",
      external: "",
    });
  const unparsedConfigurations: string | null =
    window.localStorage.getItem("configurations") === null
      ? JSON.stringify({
          recipient_ip: "",
        })
      : window.localStorage.getItem("configurations");
  const parsedConfigurations: Configurations = JSON.parse(
    unparsedConfigurations || '{"recipient_ip":""}',
  );
  const [configurations, setConfigurations] = useState<Configurations>({
    recipient_ip: parsedConfigurations.recipient_ip,
  });
  const [folders, setFolders] = useState<Folder>({
    contents: [
      {
        name: "",
        root: "",
        path: "",
        metadata: {
          is_file: false,
          file_extension: "",
        },
      },
    ],
  });
  let [notifications, setNotifications] = useState<Notifications[]>([
    {
      priority: "",
      message: "",
    },
  ]);
  let [contents, setContents] = useState<Content[]>([
    {
      name: "",
      root: "",
      path: "",
      metadata: {
        is_file: false,
        file_extension: "",
      },
    },
  ]);
  let [error, setError] = useState<ErrorBody>({
    message: "",
  });
  let [isCreateTabBtnPressed, setIsCreateTabBtnPressed] = useState(false);
  let chooseBackground: ChooseBackground[] = [
    {
      name: "Background Image 1",
      image: bg1,
    },
    // {
    //     name:"Background Image 2",
    //     image:bg2
    // }
  ];

  const [isPlaying, setIsPlaying] = useState(false);

  function play(id: string) {
    let elem: any = document.getElementById(id);
    elem.play();
    setIsPlaying(true);
  }

  function pause(id: string) {
    let elem: any = document.getElementById(id);
    elem.pause();
    setIsPlaying(false);
  }

  function toggleAudioTag(src: string) {
    setAudioSource(src);
  }

  async function open(url: string) {
    setLoadingText("Loading...");
    try {
      //setIsLoading(true)
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          root: localStorage.getItem("path"),
        }),
      });
      document.title = localStorage.getItem("path")
        ? `${localStorage.getItem("path")}`
        : "Anvel • Home";
      let path: any = localStorage.getItem("path");
      let parts = path.split("/");
      setName(parts[parts.length - 1]);
      const parseRes: any = await response.json();
      if (response.ok) {
        let withOutDotConfig: Folder = {
          contents: [],
        };
        parseRes.contents.forEach((content: Content) => {
          if (content.name.slice(0, 1) !== ".") {
            withOutDotConfig.contents.push(content);
          }
        });
        setFolders(withOutDotConfig);
        setContents(withOutDotConfig.contents);
      } else {
        setError(parseRes);
        navigate(`/error?error=${parseRes.message}`);
      }
      setIsLoading(false);
    } catch (error: any) {
      console.error(error.message);
      setIsLoading(false);
      navigate(`/error?error=${error.message}`);
    }
  }

  async function getIPs(url: string) {
    try {
      setIsLoadingNetInfo(true);
      const response = await fetch(url, {
        method: "GET",
      });
      const parseRes: any = await response.json();
      if (response.ok) {
        setNetworkInformation(parseRes);
      } else {
        setError(parseRes);
        // navigate(`/error?error=${parseRes.message}`)
      }
      setIsLoadingNetInfo(false);
    } catch (error: any) {
      console.log(error.message);
      navigate(`/error?error=${error.message}`);
    }
  }

  async function sendFile(url: string, info: SendFileInfo) {
    setLoadingText("Sending...");
    try {
      setIsLoading(true);
      let response = await fetch(url, {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          file_name: info.name,
          file_path: info.path,
          recipient_server: info.recipient_server,
        }),
      });
      let parseRes = await response.json();
      if (response.ok) {
        console.log(parseRes);
        setNotifications((prevNotifications) => [
          ...prevNotifications,
          {
            priority: "important",
            message: parseRes,
          },
        ]);
      } else {
        console.log(parseRes);
        await message(`${parseRes}`, { title: "Error", kind: "error" });
        setNotifications((prevNotifications) => [
          ...prevNotifications,
          {
            priority: "not important",
            message: `${parseRes}`,
          },
        ]);
      }
      setIsLoading(false);
    } catch (error: any) {
      console.log(error.message);
      navigate(`/error?error=${error.message}`);
    }
  }

  function onlyFolders() {
    let arr: Folder = {
      contents: [],
    };
    contents.map((content) => {
      if (!content.metadata.is_file) {
        arr.contents.push(content);
      }
      setFolders(arr);
    });
  }

  function onlyFiles() {
    let arr: Folder = {
      contents: [],
    };
    contents.map((content) => {
      if (content.metadata.is_file) {
        arr.contents.push(content);
      }
      setFolders(arr);
    });
  }

  function showToast(id: string) {
    let toast = document.getElementById(id);
    toast?.classList.contains("none")
      ? toast?.classList.remove("none")
      : toast?.classList.add("none");
  }

  function toggleDialog(id: string) {
    let dialog_bg = document.getElementById(id);
    dialog_bg?.classList.add("ease-in-out");
    dialog_bg?.classList.toggle("none");
    // dialog_bg?.classList.add("duration-1000");
    // dialog_bg?.classList.add("delay-2000");
  }

  function handleShowSettings() {
    setShowSettings(true);
    setShowSettingsTab(true);
    setStartRequestLoop(false);
    setSettingsHeader("Settings - Anvel");
    getIPs(`${API_URL}/api/get_ip_address`);
  }

  function handleCloseSettings() {
    setSettingsHeader("");
    setShowSettings(false);
  }

  function toggleShowCloseBtn(id: string) {
    let closeBtn = document.getElementById(id);
    closeBtn?.classList.contains("none")
      ? closeBtn?.classList.remove("none")
      : closeBtn?.classList.add("none");
  }

  function kickOffStartRequestLoop() {
    setStartRequestLoop(true);
  }

  function endStartRequestLoop() {
    setStartRequestLoop(false);
  }

  if (startRequestLoop === true) {
    let maxRequest = 2;
    for (let i = 0; i < maxRequest; i++) {
      setTimeout(() => {
        setCounter(i);
      }, 500);
    }
  }

  async function handlePing(e: any) {
    try {
      e.preventDefault();
      setIsDisabled(true);
      let configs: Configurations = {
        recipient_ip: e.target.recipient_ip.value,
      };
      let response = await fetch(`${API_URL}/api/ping/${configs.recipient_ip}`);
      let parseRes = await response.json();
      if (parseRes !== "pong") {
        await message(`${parseRes.error}`, { title: "Error", kind: "error" });
        console.log(parseRes.error);
        setIsDisabled(false);
      } else {
        console.log(parseRes);
        setConfigurations(configs);
        window.localStorage.setItem("configurations", JSON.stringify(configs));
        setIsDisabled(false);
      }
    } catch (error: any) {
      setIsDisabled(false);
      let errorMessage =
        error.message === "Failed to fetch"
          ? `Cannot ping ${e.target.recipient_ip.value}`
          : error.message;
      await message(`${errorMessage}`, { title: "Error", kind: "error" });
      setNotifications([
        {
          priority: "not important",
          message: errorMessage,
        },
      ]);
      console.log(errorMessage);
    }
  }

  async function getTabs() {
    try {
      const request = await indexedDb();
      const db: any = await request;
      const transaction = db.transaction("tabs", "readwrite");
      const tabStore = transaction.objectStore("tabs");

      const getTabs = tabStore.getAll();
      let tabs = [];
      getTabs.onsuccess = () => {
        tabs.push(...getTabs.result);
        setTabs(tabs);
      };

      getTabs.onerror = () => {
        console.log("error: failed to open tab", getTabs.error);
      };
    } catch (error: any) {
      console.log(error);
    }
  }

  async function createTab(name: string, path: string) {
    try {
      const request = await indexedDb();
      const db: any = await request;
      const transaction = db.transaction("tabs", "readwrite");
      const tabStore = transaction.objectStore("tabs");

      let date = new Date();
      let newObj = Intl.DateTimeFormat("en-US", {
        timeZone: "America/New_York",
      });
      let newDate = newObj.format(date);
      let min =
        date.getMinutes() < 10
          ? `0${date.getMinutes()}`
          : `${date.getMinutes()}`;
      let time =
        date.getHours() > 12
          ? `${date.getHours()}:${min}PM`
          : `${date.getHours()}:${min}AM`;
      const getTabs = tabStore.add({
        name,
        createdAt: `${newDate} ${time}`,
        path,
        type: "folder",
        id: `${Math.random()}`,
      });

      getTabs.onsuccess = () => {
        console.log("success");
        localStorage.setItem("path", path);
      };

      getTabs.onerror = () => {
        console.log("error: failed to open tab", getTabs.error);
        localStorage.setItem("path", path);
      };
    } catch (error: any) {
      console.log(error);
    }
  }

  function openNewTab() {
    setIsCreateTabBtnPressed(true);
    openDialog("open_folder_dialog");
  }

  function openFolder() {
    setIsCreateTabBtnPressed(false);
    openDialog("open_folder_dialog");
  }

  async function deleteTab(path: string) {
    try {
      const request = await indexedDb();
      const db: any = await request;
      const transaction = db.transaction("tabs", "readwrite");
      const tabStore = transaction.objectStore("tabs");

      const tabByPath = tabStore.index("path");
      const deleteTab = tabByPath.getKey([path]);

      deleteTab.onsuccess = () => {
        const del = tabStore.delete(deleteTab.result);
        del.onsuccess = () => {
          console.log("tab deleted");
          let getTabs = tabStore.getAll();
          let tabs = [];
          getTabs.onsuccess = () => {
            tabs.push(...getTabs.result);
            if (tabs.length === 0) {
              createTab("root", "root");
              console.log(tabs);
            } else {
              localStorage.getItem("path") === path
                ? localStorage.setItem("path", `${tabs[tabs.length - 1].path}`)
                : "";
            }
          };

          getTabs.onerror = () => {
            console.log("error: failed to open tab", getTabs.error);
          };
        };
        del.onerror = () => {
          console.log("error", del.result);
        };
      };
      deleteTab.onerror = () => {
        console.log("error", deleteTab.result);
      };
    } catch (error: any) {
      console.log(error.message);
    }
  }

  async function updateTab(name: string, path: string) {
    try {
      const request = await indexedDb();
      const db: any = await request;
      const transaction = db.transaction("tabs", "readwrite");
      const tabStore = transaction.objectStore("tabs");

      let oldPath = localStorage.getItem("path");
      const getTabByPath = tabStore.index("path");
      const Tab = getTabByPath.get([oldPath]);

      Tab.onsuccess = (event: any) => {
        let tabInfo = event.target.result;
        tabInfo.name = name;
        tabInfo.path = path;

        const requestUpdate = tabStore.put(tabInfo);
        requestUpdate.onerror = () => {
          console.log("error", requestUpdate.error);
          localStorage.setItem("path", path);
          //deleteTab(oldPath)
        };

        let tabs = [];
        requestUpdate.onsuccess = () => {
          // Success - the data is updated
          tabs.push(tabInfo);
          setTabs(tabs);
          localStorage.setItem("path", tabInfo.path);
          console.log("tab updated");
          showSettings === true ? setShowSettings(false) : "";
        };
      };
      Tab.onerror = () => {
        console.log("error", Tab.result);
      };
    } catch (error: any) {
      console.log(error.message);
    }
  }

  useEffect(() => {
    getTabs();
    open(`${API_URL}/api/directory_content`);
  }, [counter, tabs]);
  return (
    <>
      {isLoading ? (
        <div className="bg-[var(--primary-01)] text-[var(--primary-04)] flex flex-col h-screen w-screen items-center justify-center">
          <p className="text-lg">{loadingText}</p>
        </div>
      ) : (
        <div
          style={
            !props.data.backgroundImage.includes("primary-01") &&
            props.data.backgroundImage !== "default"
              ? {
                  background: `linear-gradient(0deg, rgba(0, 0, 0, 0.7), rgba(0, 0, 0, 0.7)),url('${props.data.backgroundImage}') top no-repeat`,
                  backgroundSize: "cover",
                  backgroundAttachment: "fixed",
                }
              : props.data.backgroundImage === "default"
                ? { background: "var(--primary-01)" }
                : { background: `var(--${props.data.backgroundImage})` }
          }
          className="min-h-[100vh]"
        >
          <TopNav
            data={{
              name,
              handleShowSettings,
              settingsHeader,
              showToast,
              openFolder,
            }}
          />
          <div className="flex">
            <SideNav
              data={{
                folders,
                error,
                open,
                getIPs,
                showSettings,
                updateTab,
                openNewTab,
                openFolder,
                tabs,
              }}
            />
            <div className="mt-[48px] flex-grow mb-[22px]">
              {/*  folder view */}
              <div id="folder_view">
                {/* folder view nav */}
                <div
                  id="folder_view_nav"
                  className="fixed overflow-hidden border-dotted border-[#3c3c3c]/50 border-l-[1px] left-[199px] right-0 top-[35px]"
                >
                  <div className="flex w-full bg-[var(--primary-02)]">
                    {localStorage.getItem("path") === "/" ? (
                      ""
                    ) : (
                      <div
                        onClick={() => {
                          let path: any =
                            localStorage.getItem("path") !== null
                              ? localStorage.getItem("path")
                              : "";
                          let newPath: any;
                          if (
                            path.slice(0, path?.lastIndexOf("/")) === "" ||
                            path
                              .slice(0, path?.lastIndexOf("/"))
                              .startsWith("\\:") ||
                            (!path
                              .slice(0, path?.lastIndexOf("/"))
                              .includes("/") &&
                              path !== "Anvel shared")
                          ) {
                            newPath = "root";
                          } else if (path === "Anvel shared") {
                            newPath = localStorage.getItem("previous");
                          } else {
                            newPath = path.slice(0, path?.lastIndexOf("/"));
                          }
                          let tabName = newPath.slice(
                            newPath?.lastIndexOf("/") + 1,
                            newPath.length,
                          );
                          updateTab(tabName, newPath);
                          open(`${API_URL}/api/directory_content`);
                          endStartRequestLoop();
                        }}
                        title="Previous"
                        className="bg-[var(--primary-02)] cursor-pointer pl-[10px] pr-[3px] w-[50px] h-[35px] flex items-center"
                      >
                        <MdArrowBack className="w-[18px] h-[18px] mr-[5px]" />
                      </div>
                    )}

                    {tabs &&
                      tabs.map((tab) => {
                        return (
                          <div
                            key={tab.name}
                            id={tab.name}
                            onMouseEnter={() =>
                              toggleShowCloseBtn(`folder_close_btn_${tab.path}`)
                            }
                            onMouseLeave={() =>
                              toggleShowCloseBtn(`folder_close_btn_${tab.path}`)
                            }
                            className={
                              showSettings === true ||
                              tab.path !== localStorage.getItem("path")
                                ? `bg-[var(--primary-02)] border-dotted border-l-[1px] border-[#3c3c3c]/50 hover:bg-[#3c3c3c]/55 cursor-pointer pr-[5px] min-w-[130px] h-[35px] flex items-center`
                                : props.data.backgroundImage !== "default"
                                  ? `bg-[var(--${props.data.backgroundImage})] hover:bg-[#3c3c3c]/55 cursor-pointer pr-[5px] min-w-[130px] h-[35px] flex items-center`
                                  : `bg-[var(--primary-01)] hover:bg-[#3c3c3c]/55 cursor-pointer pr-[5px] min-w-[130px] h-[35px] flex items-center`
                            }
                          >
                            <button
                              className="flex flex-grow  pl-[10px] h-full items-center"
                              onClick={() => {
                                localStorage.setItem("path", tab.path);
                                handleCloseSettings();
                              }}
                            >
                              <MdFolder className="w-[18px] h-[18px] mr-[5px]" />
                              <p className="mr-[3px] text-[13px] capitalize root_path_indicator">
                                {tab.name}
                              </p>
                            </button>
                            <MdClose
                              id={`folder_close_btn_${tab.path}`}
                              className="p-[3px] none w-[22px] h-[22px] bg-[var(--primary-02)] ml-auto rounded-sm"
                              onClick={() => {
                                deleteTab(tab.path);
                              }}
                            />
                          </div>
                        );
                      })}

                    {showSettingsTab ? (
                      <div
                        onMouseEnter={() =>
                          toggleShowCloseBtn(`settings_close_btn`)
                        }
                        onMouseLeave={() =>
                          toggleShowCloseBtn(`settings_close_btn`)
                        }
                        className={
                          showSettings !== true
                            ? "bg-[var(--primary-02)] border-dotted border-l-[1px] border-[#3c3c3c]/50 hover:bg-[#3c3c3c]/55 cursor-pointer pr-[3px] min-w-[128px] h-[35px] flex items-center"
                            : props.data.backgroundImage !== "default"
                              ? `bg-[var(--${props.data.backgroundImage})] hover:bg-[#3c3c3c]/55 cursor-pointer pr-[3px] min-w-[128px] h-[35px] flex items-center`
                              : `bg-[var(--primary-01)] hover:bg-[#3c3c3c]/55 cursor-pointer pr-[3px] min-w-[128px] h-[35px] flex items-center`
                        }
                      >
                        <div
                          className="flex pl-[10px]"
                          onClick={() => {
                            setSettingsHeader("Settings - Anvel");
                            setShowSettings(true);
                            setStartRequestLoop(false);
                          }}
                        >
                          <MdSettings className="w-[18px] h-[18px] mr-[5px]" />
                          <p className="mr-[3px] text-[13px] capitalize">
                            Settings
                          </p>
                        </div>
                        <MdClose
                          id="settings_close_btn"
                          className="p-[3px] none w-[22px] h-[22px] bg-[var(--primary-02)] ml-auto rounded-sm"
                          onClick={() => {
                            setShowSettings(false);
                            setShowSettingsTab(false);
                            setSettingsHeader("");
                          }}
                        />
                      </div>
                    ) : (
                      ""
                    )}
                  </div>
                </div>
                {!showSettings ? (
                  <div
                    className="w-full flex flex-wrap mt-[35px]"
                    style={
                      props.data.backgroundImage === "default" ||
                      props.data.backgroundImage.includes("primary-01")
                        ? {}
                        : { color: "white" }
                    }
                    id="folder_view_body"
                  >
                    {folders.contents.length === 0 ? (
                      <div className="ml-[200px] w-full px-[25px] py-[13px]">
                        <p
                          className="text-[13px] text-center text-[var(--primary-04)]"
                          style={
                            props.data.backgroundImage === "default" ||
                            props.data.backgroundImage.includes("primary-01")
                              ? {}
                              : { color: "white" }
                          }
                        >
                          This folder is empty
                        </p>
                      </div>
                    ) : (
                      <div
                        id="test"
                        className="ml-[200px] grid max-sm:grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 w-full gap-4 px-[25px] py-[13px]"
                      >
                        {folders.contents.map((content) => {
                          let fileIcon;
                          let downloadURL = `${API_URL}/api/download/${content.path}`;
                          switch (content.metadata.file_extension) {
                            case "apk":
                              fileIcon = APK;
                              break;
                            case "APK":
                              fileIcon = APK;
                              break;
                            case "sys":
                              fileIcon = SYS;
                              break;
                            case "exe":
                              fileIcon = EXE;
                              break;
                            case "mp3":
                              fileIcon = audioMp3;
                              break;
                            case "jpeg":
                              fileIcon = downloadURL;
                              break;
                            case "deb":
                              fileIcon = DEB;
                              break;
                            case "lnk":
                              fileIcon = LNK;
                              break;
                            case "sql":
                              fileIcon = SQL;
                              break;
                            case "db":
                              fileIcon = SQL;
                              break;
                            case "DB":
                              fileIcon = SQL;
                              break;
                            case "psd":
                              fileIcon = PSD;
                              break;
                            case "PSD":
                              fileIcon = PSD;
                              break;
                            case "JPEG":
                              fileIcon = downloadURL;
                              break;
                            case "svg":
                              fileIcon = downloadURL;
                              break;
                            case "ttf":
                              fileIcon = TXT;
                              break;
                            case "gif":
                              fileIcon = downloadURL;
                              break;
                            case "jpg":
                              fileIcon = downloadURL;
                              break;
                            case "JPG":
                              fileIcon = downloadURL;
                              break;
                            case "png":
                              fileIcon = downloadURL;
                              break;
                            case "PNG":
                              fileIcon = downloadURL;
                              break;
                            case "webp":
                              fileIcon = downloadURL;
                              break;
                            case "WEBP":
                              fileIcon = downloadURL;
                              break;
                            case "pdf":
                              fileIcon = PDF;
                              break;
                            case "iso":
                              fileIcon = ISOIMAGE;
                              break;
                            case "img":
                              fileIcon = ISOIMAGE;
                              break;
                            case "old":
                              fileIcon = OLD;
                              break;
                            case "docx":
                              fileIcon = DOCX;
                              break;
                            case "odp":
                              fileIcon = PPTX;
                              break;
                            case "html":
                              fileIcon = HTML;
                              break;
                            case "css":
                              fileIcon = CSS;
                              break;
                            case "php":
                              fileIcon = PHP;
                              break;
                            case "py":
                              fileIcon = PYTHON;
                              break;
                            case "xml":
                              fileIcon = XML;
                              break;
                            case "js":
                              fileIcon = JS;
                              break;
                            case "sh":
                              fileIcon = SCRIPT;
                              break;
                            case "odt":
                              fileIcon = DOCX;
                              break;
                            case "ini":
                              fileIcon = DESKTOP;
                              break;
                            case "ods":
                              fileIcon = XLSX;
                              break;
                            case "doc":
                              fileIcon = DOC;
                              break;
                            case "csv":
                              fileIcon = CSV;
                              break;
                            case "java":
                              fileIcon = JAVA;
                              break;
                            case "cs":
                              fileIcon = CSHARP;
                              break;
                            case "cpp":
                              fileIcon = CPP;
                              break;
                            case "rs":
                              fileIcon = TXT;
                              break;
                            case "s":
                              fileIcon = TXT;
                              break;
                            case "json":
                              fileIcon = SCRIPT;
                              break;
                            case "wav":
                              fileIcon = audioWAV;
                              break;
                            case "c":
                              fileIcon = C;
                              break;
                            case "m":
                              fileIcon = C;
                              break;
                            case "h":
                              fileIcon = CHEADER;
                              break;
                            case "rb":
                              fileIcon = RUBY;
                              break;
                            case "fal":
                              fileIcon = TXT;
                              break;
                            case "go":
                              fileIcon = TXT;
                              break;
                            case "asm":
                              fileIcon = TXT;
                              break;
                            case "nim":
                              fileIcon = TXT;
                              break;
                            case "pm":
                              fileIcon = PERL;
                              break;
                            case "pl":
                              fileIcon = PERL;
                              break;
                            case "txt":
                              fileIcon = TXT;
                              break;
                            case "md":
                              fileIcon = TXT;
                              break;
                            case "zip":
                              fileIcon = ZIP;
                              break;
                            case "mp4":
                              fileIcon = videoMp4;
                              break;
                            case "mkv":
                              fileIcon = MKV;
                              break;
                            case "avi":
                              fileIcon = AVI;
                              break;
                            case "pptx":
                              fileIcon = PPTX;
                              break;
                            case "xlsx":
                              fileIcon = XLSX;
                              break;
                            case "desktop":
                              fileIcon = DESKTOP;
                              break;
                            case "wmv":
                              fileIcon = videoWMV;
                              break;
                            default:
                              fileIcon = unknownFile;
                              break;
                          }

                          let path = content.path;
                          if (path.includes("\\")) {
                            // Replace backslashes with forward slashes
                            path = path.replace(/\\/g, "/");
                          }

                          let unique_media = ["MP3", "MP4"];
                          let label = unique_media.includes(
                            content.metadata.file_extension.toUpperCase(),
                          )
                            ? content.metadata.file_extension
                            : content.name;
                          if (label.includes(" ") || label.includes(".")) {
                            label = label.replace(/ /g, "_");
                            if (label.includes(".")) {
                              label = label.replace(/./g, "_");
                            }
                          }
                          return (
                            <div
                              key={content.name}
                              className="flex flex-col items-center text-center"
                            >
                              <button
                                id={content.name}
                                title={content.name}
                                onContextMenu={() => {
                                  let dropdown_list = document.getElementById(
                                    `context_list_${content.name}`,
                                  );
                                  dropdown_list?.classList.toggle("block");
                                }}
                                onDoubleClick={() => {
                                  if (!content.metadata.is_file) {
                                    updateTab(content.name, path);
                                    open(`${API_URL}/api/directory_content`);
                                  } else {
                                    if (
                                      browserSupportedFiles(
                                        content.metadata.file_extension,
                                      )
                                    ) {
                                      path.includes("#")
                                        ? (path = path.replace(/#/g, "%23"))
                                        : path;

                                      content.metadata.file_extension.toUpperCase() ===
                                        "MP4" ||
                                      content.metadata.file_extension.toUpperCase() ===
                                        "MKV"
                                        ? navigate(
                                            `/media?file=${path}&label=${content.name}`,
                                          )
                                        : content.metadata.file_extension.toUpperCase() ===
                                            "MP3"
                                          ? toggleAudioTag(path)
                                          : createWindow(
                                              `file://${path}`,
                                              label,
                                              content.name,
                                            );
                                    } else {
                                      openFile(`${API_URL}/api/open`, path);
                                    }
                                  }
                                }}
                                className="flex flex-col items-center justify-center text-[12px] max-w-[150px] focus:bg-[var(--primary-05)] hover:bg-[var(--primary-05)] dropdown_btn"
                              >
                                {content.metadata.is_file ? (
                                  <img
                                    src={fileIcon}
                                    alt="file"
                                    className={
                                      fileIcon !== downloadURL
                                        ? "w-[55px] h-[55px]"
                                        : "w-[75px] h-[60px] object-cover"
                                    }
                                  />
                                ) : (
                                  <img
                                    src={FolderImage}
                                    alt="folder"
                                    className="w-[65px] h-[65px]"
                                  />
                                )}
                                <div className="flex justify-center">
                                  {content.name.length < 30 ? (
                                    <p className="w-fit">{content.name}</p>
                                  ) : (
                                    <p className="w-fit">
                                      {!content.name.includes(" ")
                                        ? content.name.slice(0, 22)
                                        : content.name.slice(0, 30)}
                                      ...
                                    </p>
                                  )}
                                </div>
                              </button>
                              <div
                                id={`context_list_${content.name}`}
                                className="dropdown-content  flex-wrap  w-[200px] mt-[50px] -ml-[5px] max-lg:-ml-[27px]"
                              >
                                <div>
                                  <div
                                    onClick={() => {
                                      if (content.metadata.is_file) {
                                        if (
                                          browserSupportedFiles(
                                            content.metadata.file_extension,
                                          )
                                        ) {
                                          path.includes("#")
                                            ? (path = path.replace(/#/g, "%23"))
                                            : path;
                                          content.metadata.file_extension.toUpperCase() ===
                                            "MP4" ||
                                          content.metadata.file_extension.toUpperCase() ===
                                            "MKV"
                                            ? navigate(
                                                `/media?file=${path}&label=${content.name}`,
                                              )
                                            : content.metadata.file_extension.toUpperCase() ===
                                                "MP3"
                                              ? toggleAudioTag(path)
                                              : createWindow(
                                                  `file://${path}`,
                                                  label,
                                                  content.name,
                                                );
                                        } else {
                                          openFile(`${API_URL}/api/open`, path);
                                        }
                                      } else {
                                        updateTab(content.name, path);
                                        open(
                                          `${API_URL}/api/directory_content`,
                                        );
                                      }
                                    }}
                                    className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35 {name_str}_open_item"
                                  >
                                    <MdOpenInNew className="w-[25px] h-[25px] pr-[6px]" />
                                    <p>Open</p>
                                  </div>
                                  {content.metadata.is_file &&
                                  browserSupportedFiles(
                                    content.metadata.file_extension,
                                  ) ? (
                                    <div
                                      onClick={() => {
                                        openFile(`${API_URL}/api/open`, path);
                                      }}
                                      className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35 {name_str}_open_item"
                                    >
                                      <MdOpenInNew className="w-[25px] h-[25px] pr-[6px]" />
                                      <p>Open with other app</p>
                                    </div>
                                  ) : (
                                    ""
                                  )}

                                  {!content.metadata.is_file &&
                                  tabs &&
                                  tabs.length < 4 ? (
                                    <div
                                      onClick={() => {
                                        createTab(content.name, path);
                                      }}
                                      className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35 {name_str}_open_item"
                                    >
                                      <MdOpenInNew className="w-[25px] h-[25px] pr-[6px]" />
                                      <p>Open in a new tab</p>
                                    </div>
                                  ) : (
                                    ""
                                  )}

                                  <button
                                    onClick={() => {
                                      navigator.clipboard.writeText(path);
                                    }}
                                    className="px-[12px] w-full py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35 {name_str}_open_item"
                                  >
                                    <MdContentCopy className="w-[25px] h-[25px] pr-[6px]" />
                                    <p>Copy Path</p>
                                  </button>
                                  {content.metadata.is_file &&
                                  localStorage.getItem("path") !== "shared" ? (
                                    <div
                                      onClick={() => {
                                        if (
                                          configurations.recipient_ip.length ===
                                          0
                                        ) {
                                          handleShowSettings();
                                        } else {
                                          let sendFileInfo: SendFileInfo = {
                                            name: content.name,
                                            path,
                                            recipient_server: `http://${configurations.recipient_ip}:80/api/receive`,
                                          };
                                          sendFile(
                                            `${API_URL}/api/send`,
                                            sendFileInfo,
                                          );
                                        }
                                      }}
                                      className="px-[12px] w-full py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35 {name_str}_open_item"
                                    >
                                      <MdSend className="w-[25px] h-[25px] pr-[6px]" />
                                      <p>
                                        {configurations.recipient_ip.length !==
                                        0 ? (
                                          <span>
                                            Send to{" "}
                                            {configurations.recipient_ip}
                                          </span>
                                        ) : (
                                          <span>Add Recipient's IP</span>
                                        )}
                                      </p>
                                    </div>
                                  ) : (
                                    ""
                                  )}
                                  <button
                                    onClick={() => {
                                      toggleDialog(`file_info_dialog`);
                                      setInfoContent(content);
                                    }}
                                    className="px-[12px] w-full py-[8px] flex items-center border-t-[1px] border-[#9999991A] hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35"
                                  >
                                    <MdInfoOutline className="w-[25px] h-[25px] pr-[6px]" />
                                    <p>Properties</p>
                                  </button>
                                </div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                ) : (
                  <div
                    style={
                      props.data.backgroundImage === "default" ||
                      props.data.backgroundImage.includes("primary-01")
                        ? {}
                        : { color: "white" }
                    }
                    className="w-full flex flex-wrap mt-[35px] text-[var(--primary-04)]"
                    id="settings_view"
                  >
                    <div className="ml-[200px] flex flex-col w-full gap-x-4 gap-y-12 px-[25px] pt-[13px] pb-[50px]">
                      <div>
                        <p className="text-lg font-semibold mb-2">
                          Network Information
                        </p>
                        <div className="flex gap-6 flex-col">
                          <div>
                            <div className="flex flex-col gap-2 my-2">
                              {isLoadingNetInfo ? (
                                <div className="flex gap-2  items-center w-fit justify-center">
                                  <div className="flex items-center justify-center w-[18px] h-[18px]">
                                    <MdRefresh className="text-[14px]" />
                                  </div>
                                  <i className="text-[14px]">
                                    Searching for network information...
                                  </i>
                                </div>
                              ) : (
                                <>
                                  {networkInformation.internal.length === 0 ? (
                                    <div className="flex flex-col gap-y-4">
                                      <div className="flex gap-2  items-center w-fit justify-center">
                                        <div className="flex items-center justify-center w-[18px] h-[18px] bg-red-600 rounded-[50px] text-white">
                                          <MdClose className="text-[14px]" />
                                        </div>
                                        <p className="text-[14px]">
                                          {error.message}
                                        </p>
                                      </div>
                                      <button
                                        onClick={() => {
                                          getIPs(
                                            `${API_URL}/api/get_ip_address`,
                                          );
                                        }}
                                        className="flex items-center justify-center h-[30px] w-[120px] text-[13px] bg-[var(--primary-02)] border-none"
                                        style={{ color: "var(--primary-04" }}
                                      >
                                        Try again
                                      </button>
                                    </div>
                                  ) : (
                                    <>
                                      <div className="grid grid-cols-4 gap-10">
                                        <p>Internet Protocol (IP)</p>
                                        <p
                                          style={
                                            props.data.backgroundImage ===
                                              "default" ||
                                            props.data.backgroundImage.includes(
                                              "primary-01",
                                            )
                                              ? {}
                                              : { color: "white" }
                                          }
                                          className="text-[var(--primary-04)]"
                                        >
                                          {networkInformation.internal}
                                        </p>
                                      </div>
                                      {networkInformation.external.includes(
                                        "No internet",
                                      ) ? (
                                        ""
                                      ) : (
                                        <div className="grid grid-cols-4 gap-10">
                                          <p>External IP</p>
                                          <p
                                            style={
                                              props.data.backgroundImage ===
                                                "default" ||
                                              props.data.backgroundImage.includes(
                                                "primary-01",
                                              )
                                                ? {}
                                                : { color: "white" }
                                            }
                                            className="text-[var(--primary-04)]"
                                          >
                                            {networkInformation.external}
                                          </p>
                                        </div>
                                      )}
                                      <div className="grid grid-cols-4 gap-10">
                                        <p>Status</p>
                                        <p
                                          style={
                                            props.data.backgroundImage ===
                                              "default" ||
                                            props.data.backgroundImage.includes(
                                              "primary-01",
                                            )
                                              ? {}
                                              : { color: "white" }
                                          }
                                          className="text-[var(--primary-04)]"
                                        >
                                          {networkInformation.external.includes(
                                            "No internet",
                                          )
                                            ? "Offline"
                                            : "Online"}
                                        </p>
                                      </div>
                                    </>
                                  )}
                                </>
                              )}
                            </div>
                          </div>

                          {networkInformation.internal.length !== 0 ? (
                            <div>
                              <p className="font-semibold text-lg">
                                Recipient Information
                              </p>
                              <form
                                onSubmit={handlePing}
                                className="flex flex-col gap-2 my-2"
                              >
                                {configurations.recipient_ip.length === 0 ? (
                                  <div className="grid grid-cols-4 gap-10">
                                    <label htmlFor="recipient_ip">
                                      Enter Recipient's IP
                                    </label>
                                    <input
                                      id="recipient_ip"
                                      name="recipient_ip"
                                      className="px-2 py-1 w-full rounded-md bg-transparent border-violet-300 focus:border-none focus:outline-none border-[1px] focus:ring-1 focus:ring-violet-300"
                                      type="text"
                                      placeholder="192.10.0.95"
                                      required
                                    />
                                  </div>
                                ) : (
                                  <div className="grid grid-cols-4 gap-10">
                                    <p>Recipient's IP</p>
                                    <p
                                      className="text-[var(--primary-04)]"
                                      style={
                                        props.data.backgroundImage ===
                                          "default" ||
                                        props.data.backgroundImage.includes(
                                          "primary-01",
                                        )
                                          ? {}
                                          : { color: "white" }
                                      }
                                    >
                                      {configurations.recipient_ip}
                                    </p>
                                  </div>
                                )}
                                <div className="grid grid-cols-4 gap-10">
                                  <label htmlFor="both_folder_and_file">
                                    Send Both Folder and File
                                  </label>
                                  <input
                                    disabled
                                    id="both_folder_and_file"
                                    name="both_folder_and_file"
                                    checked
                                    type="checkbox"
                                    className="h-[20px] w-[20px] cursor-pointer rounded-md bg-transparent focus:outline-none checked:bg-violet-300 focus:ring-1 focus:ring-violet-300"
                                  />
                                </div>
                                {configurations.recipient_ip.length === 0 ? (
                                  <button
                                    disabled={isDisabled}
                                    className={
                                      !isDisabled
                                        ? "py-1 px-[16px] hover:bg-[var(--yellow-primary-02)] border-none w-[100px] text-black rounded-sm bg-[var(--yellow-primary-01)]"
                                        : "py-1 px-[16px] cursor-wait bg-[var(--yellow-primary-02)] border-none w-[100px] text-black rounded-sm"
                                    }
                                  >
                                    Ping
                                  </button>
                                ) : (
                                  <button
                                    type="button"
                                    onClick={() => {
                                      setConfigurations({
                                        recipient_ip: "",
                                      });
                                    }}
                                    className="py-1 px-[16px] hover:bg-[var(--yellow-primary-02)] border-none w-[100px] rounded-sm text-black bg-[var(--yellow-primary-01)]"
                                  >
                                    Change
                                  </button>
                                )}
                              </form>
                            </div>
                          ) : (
                            ""
                          )}
                        </div>
                      </div>

                      <div>
                        <p className="font-semibold text-lg mb-2">
                          Personalization
                        </p>
                        <div className="flex flex-col">
                          <p>Background</p>
                          <select
                            style={{ color: "var(--primary-04" }}
                            className="mt-2 active:outline-none focus:outline-none mb-4 w-[250px] border-[1px] p-[6px]"
                            onChange={(e) =>
                              setBackgroundOption(e.target.value)
                            }
                          >
                            <option value="Picture">Picture</option>
                            <option value="Solid color">Solid color</option>
                          </select>
                          {backgroundOption === "Picture" ? (
                            <>
                              <p>Choose your picture</p>
                              <div className="flex max-sm:flex-col gap-2 my-2">
                                <button
                                  onClick={() =>
                                    props.data.changeBackground("default")
                                  }
                                  style={{
                                    boxShadow:
                                      "0px 8px 16px 0px rgba(0,0,0,0.2)",
                                  }}
                                  className="bg-[var(--primary-01)] flex justify-center items-center rounded-md h-[200px] w-[240px]"
                                >
                                  <p className="text-base text-[var(--primary-04)]">
                                    Default
                                  </p>
                                </button>
                                <div className="flex max-sm:flex-col gap-2">
                                  {chooseBackground.map((choice) => {
                                    return (
                                      <button
                                        key={choice.name}
                                        onClick={() =>
                                          props.data.changeBackground(
                                            choice.image,
                                          )
                                        }
                                        style={{
                                          boxShadow:
                                            "0px 8px 16px 0px rgba(0,0,0,0.2)",
                                          background: `linear-gradient(0deg, rgba(0, 0, 0, 0.5), rgba(0, 0, 0, 0.5)),url('${choice.image}') center no-repeat`,
                                          backgroundSize: "cover",
                                        }}
                                        className={`hover:text-white flex justify-center items-center rounded-md h-[200px] w-[240px]`}
                                      >
                                        <p className="text-base text-gray-100">
                                          {choice.name}
                                        </p>
                                      </button>
                                    );
                                  })}
                                </div>
                              </div>
                            </>
                          ) : (
                            <>
                              <p>Choose your background color</p>
                              <div className="grid grid-cols-8 w-fit max-sm:grid-cols-1 gap-1 my-2">
                                <button
                                  onClick={() =>
                                    props.data.changeBackground("default")
                                  }
                                  className="bg-[var(--primary-01)] flex justify-center items-center h-[40px] w-[40px]"
                                  style={{
                                    boxShadow:
                                      "0px 8px 16px 0px rgba(0,0,0,0.1)",
                                  }}
                                ></button>
                                <button
                                  onClick={() =>
                                    props.data.changeBackground(
                                      "purple-primary-01",
                                    )
                                  }
                                  className="bg-purple-600 flex justify-center items-center h-[40px] w-[40px]"
                                  style={{
                                    boxShadow:
                                      "0px 8px 16px 0px rgba(0,0,0,0.1)",
                                  }}
                                ></button>
                                <button
                                  onClick={() =>
                                    props.data.changeBackground(
                                      "orange-primary-01",
                                    )
                                  }
                                  className="bg-orange-600 flex justify-center items-center h-[40px] w-[40px]"
                                  style={{
                                    boxShadow:
                                      "0px 8px 16px 0px rgba(0,0,0,0.1)",
                                  }}
                                ></button>
                                <button
                                  onClick={() =>
                                    props.data.changeBackground(
                                      "red-primary-01",
                                    )
                                  }
                                  className="bg-red-600 flex justify-center items-center h-[40px] w-[40px]"
                                  style={{
                                    boxShadow:
                                      "0px 8px 16px 0px rgba(0,0,0,0.1)",
                                  }}
                                ></button>
                                <button
                                  onClick={() =>
                                    props.data.changeBackground(
                                      "pink-primary-01",
                                    )
                                  }
                                  className="bg-pink-600 flex justify-center items-center h-[40px] w-[40px]"
                                  style={{
                                    boxShadow:
                                      "0px 8px 16px 0px rgba(0,0,0,0.1)",
                                  }}
                                ></button>
                              </div>
                            </>
                          )}
                        </div>
                      </div>

                      <div>
                        <p>Have a question?</p>
                        <a
                          href="https://github.com/imrany/anvel"
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-[14px] text-blue-500 active:text-gray-600"
                        >
                          Get help
                        </a>
                      </div>

                      <div>
                        <p>Help improve Anvel</p>
                        <a
                          href="mailto:imranmat254@gmail.com?subject=Feedback on Anvel"
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-[14px] text-blue-500 active:text-gray-600"
                        >
                          Give us feedback
                        </a>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
          <FileInfoDialog
            data={{ info: infoContent, functions: { toggleDialog } }}
          />
          <OpenFolderDialog
            data={{
              functions: { updateTab, open, createTab },
              isCreateTabBtnPressed,
            }}
          />
          <Footer
            data={{
              folders,
              onlyFolders,
              onlyFiles,
              open,
              handleShowSettings,
              notifications,
              showToast,
              handleCloseSettings,
              kickOffStartRequestLoop,
              endStartRequestLoop,
            }}
          />
          <ReportBugBtn data={{ status: networkInformation }} />
          <AudioTag
            data={{ audioSource, isPlaying }}
            functions={{ toggleAudioTag, play, pause }}
          />
        </div>
      )}
    </>
  );
}
