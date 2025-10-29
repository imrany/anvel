// @flow strict
import {
  MdEdit,
  MdExitToApp,
  MdNotifications,
  MdSystemUpdateAlt,
  MdOutlineExpandMore,
  MdSettings,
} from "react-icons/md";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useContext, useEffect } from "react";
import { GlobalContext } from "../context";

type Props = {
  data: {
    name: string;
    handleShowSettings: () => void;
    settingsHeader: string;
    showToast: (id: string) => void;
    openFolder: () => void;
  };
};

function TopNav(props: Props) {
  const { updateAnvel } = useContext(GlobalContext);
  window.onclick = function (event: Event) {
    if (!(event.target as Element).matches(".dropbtn")) {
      const dropdowns = document.getElementsByClassName("dropdown-content");
      let i;
      for (i = 0; i < dropdowns.length; i++) {
        const openDropdown = dropdowns[i];
        if (openDropdown.classList.contains("block")) {
          openDropdown.classList.remove("block");
        }
      }
    }
  };

  function showDropdownMenu() {
    const dropdown_list = document.getElementById("dropdown_list");
    dropdown_list?.classList.toggle("block");
  }
  useEffect(() => {
    updateAnvel();
  }, [updateAnvel]);
  return (
    <nav className="fixed bg-[var(--primary-02)] top-0 left-0 right-0 z-10">
      <div className="font-semibold text-[13px] flex justify-between h-[35px] items-center">
        <div className="dropdown">
          <button
            onClick={showDropdownMenu}
            className="pl-[12px] justify-start w-[200px] h-[35px] border-dotted border-[#3c3c3c]/50 border-r-[1px] flex dropbtn items-center py-[4px] px-[12px] cursor-pointer"
          >
            <p className="dropbtn">Anvel</p>
            <MdOutlineExpandMore className="w-[25px] h-[25px] dropbtn p-[3px]" />
          </button>
          <div id="dropdown_list" className="dropdown-content  ml-[12px]">
            <div
              onClick={props.data.openFolder}
              className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35"
            >
              <MdEdit className="w-[25px] h-[25px] pr-[6px]" />
              <p>Open Folder</p>
            </div>

            <div
              onClick={() => props.data.showToast("notification_dialog")}
              className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35"
            >
              <MdNotifications className="w-[25px] h-[25px] pr-[6px]" />
              <p>Notifications</p>
            </div>

            <div
              onClick={() => props.data.handleShowSettings()}
              className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35"
            >
              <MdSettings className="w-[25px] h-[25px] pr-[6px]" />
              <p>Settings</p>
            </div>

            <div
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
              className="px-[12px] py-[8px] flex items-center cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35"
            >
              <MdSystemUpdateAlt className="w-[25px] h-[25px] pr-[6px]" />
              <p>Update Anvel</p>
            </div>

            <div
              onClick={() => {
                localStorage.removeItem("path");
                window.location.href = "/welcome";
              }}
              className="px-[12px] py-[8px] flex items-center border-t-[1px] border-[#9999991A] cursor-pointer hover:bg-[#3c3c3c]/35 active:bg-[#3c3c3c]/35"
            >
              <MdExitToApp className="w-[25px] h-[25px] pr-[6px]" />
              <p>Exit</p>
            </div>
          </div>
        </div>
        <div className="h-[35px] justify-center border-[#3c3c3c]/50 border-b-[1px] gap-x-2 py-1 flex-grow flex font-medium items-center">
          {props.data.settingsHeader.length !== 0 ? (
            <p className="capitalize">{props.data.settingsHeader}</p>
          ) : (
            <>
              <p className="rounded-md text-[var(--primary-01)] bg-[var(--primary-04)] py-[2px] px-2">
                Directory
              </p>
              <p className="capitalize">{props.data.name}</p>
            </>
          )}
        </div>
        {/* <div className="text-[#C2C2C2] flex gap-2 min-w-[10vw] justify-around">
                    more nav link or btn
                </div> */}
      </div>
    </nav>
  );
}

export default TopNav;
