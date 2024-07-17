import { invoke } from "@tauri-apps/api/tauri";
import { message } from "@tauri-apps/api/dialog";
import indexedDb from "./indexedDb";

export async function createWindow(filePath:string, label:string, title:string){
    try{
        let open=await invoke("open_window", { filePath, label, title })
        console.log(open)
    }catch(error:any){
        console.log(error)
        await message(error,{title:`Error`,type:"error"})
    }
}

export function browserSupportedFiles(extension:string){
    let $extension=extension.toUpperCase();
    let supportedFileExts:string[]=["MP4","PDF","JPG","JPEG","SVG","GIF","PNG","JSON","TXT","CSV","MP3","WEBP","HTML","CSS","JS","PHP","XML"]
    if(supportedFileExts.includes($extension)){
        return true
    }else{
        return false
    }
}

export function openDialog(dialog_id:string){
    let dialog_bg=document.getElementById(dialog_id);
    dialog_bg?.classList.add("ease-in-out");
    dialog_bg?.classList.toggle("none");
    dialog_bg?.classList.add("duration-1000");
    dialog_bg?.classList.add("delay-2000"); 
}

export async function openFile(url:string,path:string){
    try {
        const response=await fetch(url,{
            method:"POST",
            headers:{
                "content-type":"application/json"
            },
            body:JSON.stringify({
                root:path
            })
        })
        const parseRes:any=await response.json()
        if(!response.ok){
            return parseRes
        }
    } catch (error:any) {
        return error
    }
}

export async function createTab(name:string,path:string){
        try{
            const request=await indexedDb()
            const db:any=await request
            const transaction=db.transaction("tabs","readwrite")
            const tabStore=transaction.objectStore("tabs")

            let date=new Date()
            let newObj = Intl.DateTimeFormat('en-US', {
                timeZone: "America/New_York"
            })
            let newDate = newObj.format(date);
            let min=date.getMinutes()<10?`0${date.getMinutes()}`:`${date.getMinutes()}`
            let time=date.getHours()>12?`${date.getHours()}:${min}PM`:`${date.getHours()}:${min}AM`
            const getTabs=tabStore.add({
                name,
                createdAt:`${newDate} ${time}`,
                path,
                type:"folder",
                id:`${Math.random()}`
            })
                                                                            
            getTabs.onsuccess=()=>{                
                console.log("success")
                localStorage.setItem("path",path);
            }
                            
            getTabs.onerror=()=>{
                console.log("error: failed to open tab",getTabs.error)
                localStorage.setItem("path",path)
            }
        }catch(error:any){
            console.log(error)
        }
    }

