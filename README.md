# TODOS
-  a basic http server ✅
-  handler guard ✅
-  handler shoule be `(req) => Resule(res,err)` ✅
-  clear all unwrap!
-  add method route support! ✅
-  shared values( no need ) ✅
-  using Builder to create server ✅
-  File Transfor ✅
-  static mapping~ ✅
-  basic httpserver ✅
-  dynamic route matching ✅
-  suppor for json inbound and outbound ✅
-  pathparam ✅
-  add `_req:HttpRequest` param for handler! ✅
-  add `mount('/',handlers!(..))`✅
-  add error information for dynamic route defination ✅
-  implememt `HttpResponseModifier` for some basic types 👷
-  serachParam ✅
-  many guard could share one route!✅
-  multipart!!!✅
-  multipart Option support ✅
-  merge macro and lib together ✅
-  add cookieModifier
-  add cookie access to request ✅
-  add new struct `FromRequest`, anything but searchParam,pathParams,json, and multiPart showing in the args of a handler, can be parsed from request ✅
-  optimise `ConvertFromRefString` to `TryConvertFrom` and `TryConvertInto` ✅
-  refactor multipart file.. save every part as file, and only keep the path to that file, when access the field, just read the file again and process parsing.✅
    1. using fixed buff to parse the html body. ✅
    2. save every part to file,and keep the file name. ✅
    3. when access using path to access it!  ✅
    > things to change `TryFromMultipartDataMap` `Part` ✅

-  support `Option` for **searchParam** in handler args✅
-  Error!!! ✅
-  add vec support for multipart ✅
-  Tls ✅
-  http2 ✅
-  WebSocket Support for h2 and http1.1 ✅
-  Further improve the implementation of WS ✅
-  add type abstraction for handler and guard ✅

- impl `deref` for `Json` ✅
- add comment for `static_map` ✅
- static_map need urldecode! ✅
- improve `handlers` macro ✅ now support `path` param 
- global error handler!✅
- add a `TryFromHttpRequest` to replace TryFrom<&mut HttpRequset>✅
