// @flow strict
import { MdEdit, MdFileOpen, MdFolder, MdMoreHoriz, MdRefresh, MdSearch } from "react-icons/md"
import { openDialog, createWindow } from "./actions"
import { ErrorBody, Folder } from "../types/definitions"
import { useState, useContext } from "react";
import { GlobalContext } from "../context"

type Props = {
    data:{
        folders:Folder,
        error:ErrorBody
        open:any,
        getIPs:any,
        showSettings:boolean
    }
};
function SideNav(props:Props) {
    let { API_URL }=useContext(GlobalContext)
    let [searchView,setSearchView]=useState(false)
    let [searchError,setSearchError]=useState(<></>)
    let [searchResults,setSearchResults]=useState<Folder>({
        contents:[]
    })
    function handleSearch(e:any){
        e.preventDefault()
        let input=e.target.value
        let results:Folder={
            contents:[]
        }
        props.data.folders.contents.forEach((content)=>{
            if(content.name.includes(input)){
                results.contents.push(content)
            }else{
                setSearchError(<>
                <p>Cannot found "{input.length>9?(<>{input.slice(0,7)}...</>):input}"</p>
                <p className="font-semibold mt-2">Note :</p>
                <ul style={{listStyleType:"initial", marginLeft:20}}>
                    <li>Try searching with keywords.</li>
                    <li>Consider case sensitive words.</li>
                </ul>
                </>)
            }
        })
        setSearchResults(results)
    }
    return (
        <div id="sidebar" className="overflow-hidden bg-[var(--primary-02)] border-dotted border-[#3c3c3c]/50 border-r-[1px] h-[100vh] fixed pb-[12px] bottom-[18px] left-0 w-[200px] top-[35px] text-[13px]">
            <div className="flex flex-col mb-3">
                <div className="h-[46px] flex items-center uppercase pl-[12px] pr-[8px]">
                    <button  onClick={()=>{
                        setSearchResults({
                            contents:[]
                        })
                        setSearchError(<></>)
                        setSearchView(false)
                    }} className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer  p-[4px]">
                        <MdFileOpen className="w-[18px] h-[18px]"/>
                    </button>
                    <button onClick={()=>setSearchView(true)} className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer p-[4px]">
                        <MdSearch className="w-[18px] h-[18px]"/>
                    </button>
                    <button onClick={()=>{
                        props.data.showSettings===false?props.data.open(`${API_URL}/api/directory_content`):props.data.getIPs(`${API_URL}/api/get_ip_address`)
                    }} className="focus:ring-1 focus:ring-violet-300 rounded-sm cursor-pointer p-[4px]">
                        <MdRefresh className="w-[18px] h-[18px]"/>
                    </button>
                </div>
                {/* folders */}
                {searchView?"":(
                    <div className="resize-y">
                        <div className="flex items-center text-[11px] uppercase px-[8px] h-[35px]">
                            <p className="pl-[12px]">EXPLORER</p>
                            <MdMoreHoriz className="w-[30px] ml-auto h-[25px] cursor-pointer p-[4px]"/>
                        </div>
                        <div id="folders" className="sidebar_folders overflow-y-auto pb-[33px] pt-1 h-screen">
                            <div className="flex flex-col">
                                {props.data.folders?props.data.folders.contents.map(content=>{
			    	                let path=content.path
		         	                if(path.includes("\\")){
			                            // Replace backslashes with forward slashes
            			                path = path.replace(/\\/g, "/")
        			                }

                                    let label=content.name
                                    if(label.includes(" ")){
                                        label=label.replace(/ /g,"_")
                                        if(label.includes(".")){
                                            label=label.replace(/./g,"_")
                                        }
                                    }
                                    return(
                                        <div className="flex-grow" key={content.name} title={content.name}>
                                            {content.metadata.is_file?(
                                                <button key={content.name} onClick={()=>{
                                                    if(!content.metadata.is_file){
                                                        localStorage.setItem("path",path)
                                                        props.data.open(`${API_URL}/api/directory_content`)
                                                    }else{
                                                        createWindow(`file://${path}`,label)
                                                        //openFile(`${API_URL}/api/open`,path)
                                                    }
                                                }} className='flex w-[195px] items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300'>
                                                    <MdFileOpen className="w-[20px] h-[20px] pr-[3px]"/>
                                                    <p className='text-[11px] uppercase'>{content.name.length<20?content.name:(<>{content.name.slice(0,18)}...</>)}</p>
                                                </button>
                                            ):(
                                                <button onClick={()=>{
                                                    localStorage.setItem("path",path)
                                                    props.data.open(`${API_URL}/api/directory_content`)
                                                }} key={content.name} className='flex w-[195px] flex-grow items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300'>
                                                    <MdFolder className="w-[20px] h-[20px] pr-[3px]"/>
                                                    <p className='text-[11px] uppercase'>{content.name.length<20?content.name:(<>{content.name.slice(0,18)}...</>)}</p>
                                                </button>
                                            )}
                                        </div>
                                    )
                                }):(
                                    <div className="flex flex-col justify-start items-start py-2 px-3">
                                        <p>{props.data.error.message}</p>
                                        <button onClick={()=>openDialog("open_folder_dialog")} className="mt-2 underline flex gap-2 text-blue-500 items-center justify-center">
                                            <MdEdit className="w-[16px] h-[16px]"/>
                                            <span>Edit path</span>
                                        </button>
                                    </div>
                                )}
                            </div>
                        </div>
                    </div>
                )}

                {/* search */}
                {searchView?(
                    <div id="search" className="resize-y">
                         <div className="flex items-center text-[11px] uppercase px-[8px] h-[35px]">
                             <input onChange={handleSearch} type="text" className="px-2 py-[3px] border-[1px] border-violet-300 text-[12px] w-full placeholder:text-gray-800 bg-[var(--primary-02)] focus:outline-[1px] focus:border-none focus:outline-none focus:outline-violet-300" placeholder="Search..."/>
                        </div>
                        <div id="folders" className="sidebar_folders overflow-y-auto pb-[33px] pt-1 h-screen">
                            <div className="flex flex-col">
                                {searchResults.contents.length!==0?searchResults.contents.map(content=>{
                                    let path=content.path
                                    if(path.includes("\\")){
                                        // Replace backslashes with forward slashes
                                        path = path.replace(/\\/g, "/")
                                    }

                                    let label=content.name
                                    if(label.includes(" ")){
                                        label=label.replace(/ /g,"_")
                                        if(label.includes(".")){
                                            label=label.replace(/./g,"_")
                                        }
                                    }
                                   return(
                                        <div className="flex-grow" key={content.name} title={content.name}>
                                            {content.metadata.is_file?(
                                                <button key={content.name} onClick={()=>{
                                                    if(!content.metadata.is_file){
                                                        localStorage.setItem("path",path)
                                                        props.data.open(`${API_URL}/api/directory_content`)
                                                    }else{
                                                        createWindow(`file://${path}`,label)
                                                        //openFile(`${API_URL}/api/open`,content.path)
                                                    }
                                                }} className='flex w-[195px] flex-grow items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300'>
                                                    <MdFileOpen className="w-[20px] h-[20px] pr-[3px]"/>
                                                    <p className='text-[11px] uppercase'>{content.name.length<20?content.name:(<>{content.name.slice(0,18)}...</>)}</p>
                                                </button>
                                            ):(
                                                <button onClick={()=>{
                                                    localStorage.setItem("path",path)
                                                    props.data.open(`${API_URL}/api/directory_content`)
                                                }} key={content.name} id='folders_{name_str}' className='flex w-[195px] flex-grow items-center mx-[1px] px-3 py-1 cursor-pointer focus:ring-1 focus:ring-violet-300'>
                                                    <MdFolder className="w-[20px] h-[20px] pr-[3px]"/>
                                                    <p className='text-[11px] uppercase'>{content.name.length<20?content.name:(<>{content.name.slice(0,18)}...</>)}</p>
                                                </button>
                                            )}
                                        </div>
                                    )
                                }):(
                                    <div className="flex flex-col justify-start items-start py-2 px-3">
                                        <p>{searchError}</p>
                                    </div>
                                )}
                            </div>
                        </div>
                    </div>
                ):""}
                
            </div>
        </div>
    );
};

export default SideNav;
