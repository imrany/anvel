import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import ErrorPage from "./pages/ErrorPage";
import Docs from "./pages/Docs";
import Home from "./pages/Home";
import Media from "./pages/Media";
import LandingPage from "./pages/LandingPage";
import Layout from "./pages/Layout";
import { useState, useEffect } from "react";
import { UserPreference } from "./types/definitions";
import { invoke } from "@tauri-apps/api/core";
import { socket } from "./ws";
import { GlobalContext } from "./context";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

function App() {
  const API_URL = `http://localhost:80`;
  const [ws, setWs] = useState<WebSocket | null>(null);
  const userPreference: UserPreference = {
    backgroundImage: "default",
  };
  const user_preference: string | null =
    localStorage.getItem("user_preference") !== null
      ? localStorage.getItem("user_preference")
      : "";
  const userPreferenceParsed: UserPreference =
    user_preference && user_preference.length !== 0
      ? JSON.parse(user_preference)
      : userPreference;
  const [backgroundImage, setBackgroundImage] = useState(
    userPreferenceParsed.backgroundImage,
  );
  const path = localStorage.getItem("path");

  function changeBackground(background: string) {
    userPreference["backgroundImage"] = background;
    localStorage.setItem("user_preference", JSON.stringify(userPreference));
    setBackgroundImage(background);
  }

  window.oncontextmenu = (e: Event) => {
    e.preventDefault();
  };

  async function startAnvel() {
    await invoke("serve_anvel");
  }

  async function updateAnvel() {
    try {
      const update = await check();
      if (update) {
        console.log(
          `found update ${update.version} from ${update.date} with notes ${update.body}`,
        );
        let downloaded = 0;
        let contentLength = 0;
        // alternatively we could also call update.download() and update.install() separately
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case "Started":
              contentLength = event.data.contentLength as number;
              console.log(
                `started downloading ${event.data.contentLength} bytes`,
              );
              break;
            case "Progress":
              downloaded += event.data.chunkLength;
              console.log(`downloaded ${downloaded} from ${contentLength}`);
              break;
            case "Finished":
              console.log("download finished");
              break;
          }
        });

        console.log("update installed");
        await relaunch();
      }
    } catch (error) {
      console.error(error);
      const updateAnvelElem = document.querySelectorAll("#update_anvel");
      updateAnvelElem.forEach((elem) => {
        elem?.classList.add("none");
      });
    }
  }

  useEffect(() => {
    startAnvel();
    socket.onopen = () => {
      console.log("WebSocket connection established", socket);
      setWs(socket);
    };

    socket.onmessage = (event: MessageEvent) => {
      console.log(event);
    };

    socket.onopen = () => {
      console.log("WebSocket connection established");
      setWs(socket);
    };
  }, []);

  //function sendMessage(){
  //if(ws){
  //   ws.send("hello")
  //}
  //}
  return (
    <BrowserRouter>
      <GlobalContext.Provider value={{ ws, API_URL, updateAnvel }}>
        <Routes>
          <Route
            path="/welcome"
            element={
              path === null ? (
                <LandingPage data={{ backgroundImage }} />
              ) : (
                <Navigate to="/" />
              )
            }
          />
          <Route
            path="/"
            element={path !== null ? <Layout /> : <Navigate to="/welcome" />}
          >
            <Route
              index
              element={<Home data={{ backgroundImage, changeBackground }} />}
            />
            <Route path="docs" element={<Docs />} />
            <Route path="media" element={<Media />} />
          </Route>
          <Route path="*" element={<ErrorPage />} />
        </Routes>
      </GlobalContext.Provider>
    </BrowserRouter>
  );
}

export default App;
