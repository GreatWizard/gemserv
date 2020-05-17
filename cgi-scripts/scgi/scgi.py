#! /usr/local/bin/python3.6
import scgi
import scgi.scgi_server

class TimeHandler(scgi.scgi_server.SCGIHandler):
    def produce(self, env, bodysize, input, output):
       header = "20\ttext/gemini\r\n"
       hi = "python\n"
       output.write(header.encode())
       output.write(hi.encode())

    # Main program: create an SCGIServer object to
    # listen on port 4000.  We tell the SCGIServer the
    # handler class that implements our application.
server = scgi.scgi_server.SCGIServer(
    handler_class=TimeHandler,
    port=4000
                        )
    # Tell our SCGIServer to start servicing requests.
    # This loops forever.
server.serve()
